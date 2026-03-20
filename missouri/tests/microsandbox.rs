//! Integration tests for the microsandbox backend.
//!
//! These tests require a running microsandbox server at 127.0.0.1:5555.
//! They are skipped if the server is not available.

use std::process::Command;

use camino::Utf8PathBuf;

fn missouri_bin() -> Utf8PathBuf {
    Utf8PathBuf::try_from(assert_cmd::cargo_bin!("missouri").to_path_buf()).unwrap()
}

fn msb_server_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5555".parse().unwrap(),
        std::time::Duration::from_millis(100),
    )
    .is_ok()
}

fn fixture_dir() -> Utf8PathBuf {
    Utf8PathBuf::from(format!(
        "{}/tests/missouri-msb",
        env!("CARGO_MANIFEST_DIR")
    ))
}

#[test]
fn microsandbox_echo_runs_in_linux_vm() {
    if !msb_server_available() {
        eprintln!("skipping: msb server not running on :5555");
        return;
    }

    let output = Command::new(missouri_bin().as_str())
        .args(["run", "-d", fixture_dir().as_str(), "-v"])
        .output()
        .expect("failed to run missouri");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "missouri run failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("hello-from-microsandbox"),
        "expected echo output in stdout: {stdout}"
    );
}

#[test]
fn microsandbox_uname_proves_linux_execution() {
    if !msb_server_available() {
        eprintln!("skipping: msb server not running on :5555");
        return;
    }

    let output = Command::new(missouri_bin().as_str())
        .args(["run", "-d", fixture_dir().as_str(), "-v"])
        .output()
        .expect("failed to run missouri");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The uname transition should produce "Linux" — proving execution
    // happens inside a Linux microVM, not on the macOS host.
    assert!(
        stdout.contains("Linux"),
        "expected 'Linux' from uname in stdout: {stdout}"
    );
}
