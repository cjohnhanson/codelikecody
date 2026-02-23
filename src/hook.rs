use std::io::Read;

use serde_json::Value;

use crate::adapter::Adapter;
use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::error::Error;
use crate::event::Response;

/// Run the hook: read JSON from stdin, process event, write response to stdout.
/// Returns the exit code to use.
pub fn run() -> Result<i32, Error> {
    let input = read_stdin()?;
    let json: Value = serde_json::from_str(&input)
        .map_err(|e| Error::NonBlocking(format!("invalid JSON on stdin: {e}")))?;

    // For now, always use the Claude Code adapter.
    // Future: detect adapter from config or input shape.
    let adapter = ClaudeCodeAdapter;

    let event = adapter.parse_event(&json)?;

    // For now, the event system just passes everything through.
    // Future tiskets (worktree-guard, phase-enforcement, etc.) add real logic here.
    let response = handle_event(&event);

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

fn handle_event(event: &crate::event::Event) -> Response {
    use crate::event::Event;

    match event {
        Event::SessionStarting { .. } => Response::Allow {
            context: Some("clc is active.".to_string()),
        },
        _ => Response::Passthrough,
    }
}
