use serde_json::Value;

use crate::event::{Event, Response};
use crate::git::GitState;
use crate::phase::Phase;

/// Read-only tools: always allowed regardless of branch or phase.
const READ_ONLY_TOOLS: &[&str] = &["Read", "Glob", "Grep"];

/// Tools that target a file via `file_path` in `tool_input`.
const FILE_TARGETING_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];

/// Evaluate an event against the current git state and phase.
pub fn evaluate(event: &Event, git: Option<&GitState>, phase: Option<Phase>) -> Response {
    match event {
        Event::PreToolUse {
            tool_name,
            tool_input,
        } => check_tool_use(tool_name, tool_input, git, phase),
        Event::Stop => check_stop(git, phase),
        _ => Response::Passthrough,
    }
}

fn check_stop(git: Option<&GitState>, phase: Option<Phase>) -> Response {
    let Some(state) = git else {
        return Response::Passthrough;
    };

    if state.is_main {
        return Response::Passthrough;
    }

    let Some(current_phase) = phase else {
        return Response::Passthrough;
    };

    match current_phase {
        Phase::Done | Phase::Green => Response::Passthrough,
        _ => Response::Block {
            message: format!(
                "Work is not complete. Current phase: {current_phase}. \
                 Run `clc done` to finalize, or `clc status set green` when tests pass."
            ),
        },
    }
}

fn check_tool_use(
    tool_name: &str,
    tool_input: &Value,
    git: Option<&GitState>,
    phase: Option<Phase>,
) -> Response {
    let Some(state) = git else {
        return Response::Passthrough;
    };

    // Main branch guard: block file-writing tools only.
    if state.is_main && FILE_TARGETING_TOOLS.contains(&tool_name) {
        return Response::Block {
            message: format!(
                "Blocked: {tool_name} is not allowed on trunk.\n\
                 File modifications are not permitted on the main branch.\n\
                 Pick up a tisket to begin work: `clc pickup <issue-id>`"
            ),
        };
    }

    // Feature branch: read-only tools always pass.
    if READ_ONLY_TOOLS.contains(&tool_name) {
        return Response::Passthrough;
    }

    // Feature branch phase enforcement.
    let Some(current_phase) = phase else {
        // No phase set — allow everything (pre-phase workflow).
        return Response::Passthrough;
    };

    match current_phase {
        Phase::Implementing => Response::Passthrough,
        Phase::TestsUnwritten | Phase::TestsWritten | Phase::Red | Phase::Green | Phase::Done => {
            check_phase_restricted(tool_name, tool_input, current_phase)
        }
    }
}

/// In restricted phases, only edits targeting tests/missouri/ are allowed.
fn check_phase_restricted(tool_name: &str, tool_input: &Value, phase: Phase) -> Response {
    if FILE_TARGETING_TOOLS.contains(&tool_name) {
        let file_path = tool_input
            .get("file_path")
            .and_then(Value::as_str)
            .unwrap_or("");

        if is_test_path(file_path) {
            return Response::Passthrough;
        }

        return Response::Block {
            message: format!(
                "Blocked: {tool_name} targeting '{file_path}' is not allowed in phase '{phase}'.\n\
                 Only edits in tests/missouri/ are permitted in this phase.\n\
                 Use `clc status set implementing` to unlock all edits."
            ),
        };
    }

    // Non-file-targeting write tools (Bash, Task, etc.) — allow in restricted phases.
    // Bash is hard to gate by path, and blocking it entirely would be too restrictive.
    Response::Passthrough
}

fn is_test_path(path: &str) -> bool {
    // Match both relative and absolute paths containing tests/missouri/
    path.contains("tests/missouri/") || path.starts_with("tests/missouri")
}
