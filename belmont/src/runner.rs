use std::collections::BTreeMap;
use std::io::Read;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::error::Result;
use crate::scrub::Scrubber;

/// Run a command inside a PTY with secrets injected as environment variables
/// and output scrubbed of secret values.
///
/// Returns the subprocess exit code.
pub fn run_command(
    command: &[String],
    secrets: BTreeMap<String, String>,
    scrubber: &mut Scrubber,
) -> Result<i32> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(command.join(" "));

    for (name, value) in &secrets {
        cmd.env(name, value);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    // Drop slave so we get EOF when the child exits.
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    // Read in chunks, scrub, and write to stdout.
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&buf[..n]);
                let scrubbed = scrubber.feed(&chunk);
                if !scrubbed.is_empty() {
                    print!("{scrubbed}");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(_) => break,
        }
    }

    // Flush remaining buffered content.
    let remaining = scrubber.flush();
    if !remaining.is_empty() {
        print!("{remaining}");
    }

    let status = child
        .wait()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    Ok(match status.exit_code() {
        code => code as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with_secrets(
        command: &str,
        secrets: Vec<(&str, &str)>,
    ) -> (String, i32) {
        let secret_map: BTreeMap<String, String> = secrets
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let entries: Vec<(String, String)> = secrets
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let mut scrubber = Scrubber::new(entries);

        // Capture stdout by running in a way we can collect output.
        // For testing, we reimplement the core loop to collect into a string
        // instead of printing.
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(command);
        for (name, value) in &secret_map {
            cmd.env(name, value);
        }

        let mut child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut output = String::new();
        let mut buf = [0u8; 4096];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    let scrubbed = scrubber.feed(&chunk);
                    output.push_str(&scrubbed);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(_) => break,
            }
        }
        output.push_str(&scrubber.flush());

        let status = child.wait().unwrap();
        (output, status.exit_code() as i32)
    }

    #[test]
    fn echoed_secret_is_scrubbed() {
        let (output, code) = run_with_secrets(
            "echo $MY_SECRET",
            vec![("MY_SECRET", "super-secret-value")],
        );
        assert_eq!(code, 0);
        assert!(
            output.contains("belmont://MY_SECRET"),
            "expected scrubbed reference, got: {output}"
        );
        assert!(
            !output.contains("super-secret-value"),
            "secret value leaked in output: {output}"
        );
    }

    #[test]
    fn exit_code_propagated() {
        let (_, code) = run_with_secrets("exit 42", vec![]);
        assert_eq!(code, 42);
    }

    #[test]
    fn no_secrets_passthrough() {
        let (output, code) = run_with_secrets("echo hello world", vec![]);
        assert_eq!(code, 0);
        assert!(
            output.contains("hello world"),
            "expected output, got: {output}"
        );
    }

    #[test]
    fn multiple_secrets_scrubbed() {
        let (output, code) = run_with_secrets(
            "echo \"$SECRET_A and $SECRET_B\"",
            vec![("SECRET_A", "alpha123"), ("SECRET_B", "beta456")],
        );
        assert_eq!(code, 0);
        assert!(
            !output.contains("alpha123"),
            "SECRET_A leaked: {output}"
        );
        assert!(
            !output.contains("beta456"),
            "SECRET_B leaked: {output}"
        );
        assert!(
            output.contains("belmont://SECRET_A"),
            "missing scrubbed reference for SECRET_A: {output}"
        );
        assert!(
            output.contains("belmont://SECRET_B"),
            "missing scrubbed reference for SECRET_B: {output}"
        );
    }

    #[test]
    fn secret_in_stderr_also_scrubbed() {
        // PTY merges stdout and stderr, so stderr secrets should also be scrubbed
        let (output, code) = run_with_secrets(
            "echo $MY_SECRET >&2",
            vec![("MY_SECRET", "stderr-secret")],
        );
        assert_eq!(code, 0);
        assert!(
            !output.contains("stderr-secret"),
            "secret leaked in stderr: {output}"
        );
    }

    #[test]
    fn command_failure_returns_nonzero() {
        let (_, code) = run_with_secrets("false", vec![]);
        assert_ne!(code, 0);
    }
}
