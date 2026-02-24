use serde_json::Value;

use crate::event::{Event, Response};
use crate::git::GitState;
use crate::phase::Phase;

const ALLOWED_ON_MAIN: &[&str] = &["Read", "Glob", "Grep"];

/// Read-only tools that are always allowed regardless of phase.
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
        Event::SessionStart { .. } => session_context(git),
        _ => Response::Passthrough,
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

    // Main branch guard: only read tools allowed.
    if state.is_main {
        if ALLOWED_ON_MAIN.contains(&tool_name) {
            return Response::Passthrough;
        }
        return Response::Block {
            message: format!(
                "Blocked: {tool_name} is not allowed on the main branch.\n\
                 Only read operations (Read, Glob, Grep) are permitted on main.\n\
                 Create a worktree to make changes: git worktree add .worktrees/<name> -b <branch>"
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

fn session_context(git: Option<&GitState>) -> Response {
    let Some(state) = git else {
        return Response::Allow {
            context: Some("clc is active. No git repository detected.".to_string()),
        };
    };

    if state.is_main {
        Response::Allow {
            context: Some(format!(
                "clc is active. On branch '{}' (main). \
                 Write operations are blocked. \
                 Pick up a tisket and create a worktree to begin work.",
                state.branch
            )),
        }
    } else {
        Response::Allow {
            context: Some(format!("clc is active. On branch '{}'.", state.branch)),
        }
    }
}
