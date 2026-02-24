use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::adapter::Adapter;
use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::config;
use crate::error::Error;
use crate::event::Response;
use crate::git;
use crate::guard;
use crate::phase;

/// Run the hook: read JSON from stdin, process event, write response to stdout.
/// Returns the exit code to use.
pub fn run() -> Result<i32, Error> {
    let input = read_stdin()?;
    let json: Value = serde_json::from_str(&input)
        .map_err(|e| Error::NonBlocking(format!("invalid JSON on stdin: {e}")))?;

    let adapter = ClaudeCodeAdapter;
    let event = adapter.parse_event(&json)?;

    let cwd = json
        .get("cwd")
        .and_then(Value::as_str)
        .map_or_else(|| Path::new("."), Path::new);
    let cfg = config::load(cwd).unwrap_or_default();
    let git_state = git::detect(cwd, &cfg.main_branch);
    let current_phase = phase::load(cwd).unwrap_or(None);

    let response = guard::evaluate(&event, git_state.as_ref(), current_phase);

    let (output, exit_code) = adapter.format_response(&event, &response);

    if let Some(json_output) = output {
        let formatted = serde_json::to_string(&json_output)
            .map_err(|e| Error::NonBlocking(format!("failed to serialize response: {e}")))?;
        println!("{formatted}");
    }

    if let Response::Block { ref message } = response {
        eprintln!("{message}");
    }

    Ok(exit_code)
}

fn read_stdin() -> Result<String, Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::NonBlocking(format!("failed to read stdin: {e}")))?;
    Ok(buf)
}
