use serde_json::Value;

use crate::event::{Event, Response};
use crate::git::GitState;
use crate::phase::Phase;

/// Read-only tools: always allowed regardless of branch or phase.
const READ_ONLY_TOOLS: &[&str] = &["Read", "Glob", "Grep"];

/// Tools that target a file via `file_path` in `tool_input`.
const FILE_TARGETING_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];

/// Bash command prefixes allowed on trunk. Anything not matching is blocked.
/// Conservative: false positives are better than writes on trunk.
const BASH_ALLOWLIST: &[&str] = &[
    "git ",
    "git\n",
    "cargo test",
    "cargo clippy",
    "cargo fmt --check",
    "cargo check",
    "cargo build",
    "clc ",
    "clc\n",
    "missouri ",
    "missouri\n",
    "tisket issue list",
    "tisket issue show",
    "tisket issue path",
    "tisket search",
    "ls",
    "pwd",
    "which ",
    "cat ",
    "head ",
    "tail ",
    "wc ",
    "find ",
    "tree ",
];

/// Evaluate an event against the current git state and phase.
pub fn evaluate(event: &Event, git: Option<&GitState>, phase: Option<Phase>) -> Response {
    // Escape hatch: set CLC_GUARD_OFF=1 to bypass all guard checks.
    // Used during clc development when the guard itself is being modified.
    if std::env::var("CLC_GUARD_OFF").is_ok() {
        return Response::Passthrough;
    }
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

    let message = match phase {
        None => "No phase set — work has not started. \
                 Set a phase with `clc status set tests-unwritten` or run `clc pickup`."
            .to_string(),
        Some(Phase::Done | Phase::Green) => return Response::Passthrough,
        Some(current_phase) => format!(
            "Work is not complete. Current phase: {current_phase}. \
             Run `clc done` to finalize, or `clc status set green` when tests pass."
        ),
    };

    Response::Block { message }
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

    // Main branch guard: only read-only tools and allowlisted Bash pass through.
    if state.is_main {
        if READ_ONLY_TOOLS.contains(&tool_name) {
            return Response::Passthrough;
        }

        if tool_name == "Bash" {
            return check_bash_allowlist(tool_input);
        }

        // Everything else (Edit, Write, NotebookEdit, Task, etc.) blocked on trunk.
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
    // No phase = restrictive (same as tests-unwritten). The agent must set a phase
    // before making non-test edits.
    let current_phase = phase.unwrap_or(Phase::TestsUnwritten);

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

/// Check if a Bash command is on the trunk allowlist.
fn check_bash_allowlist(tool_input: &Value) -> Response {
    let command = tool_input
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("");

    let trimmed = command.trim_start();

    for prefix in BASH_ALLOWLIST {
        if trimmed.starts_with(prefix) || trimmed == prefix.trim() {
            return Response::Passthrough;
        }
    }

    Response::Block {
        message: format!(
            "Blocked: Bash command is not allowed on trunk.\n\
             Only read-only commands are permitted on the main branch.\n\
             Command: {}\n\
             Pick up a tisket to begin work: `clc pickup <issue-id>`",
            truncate_command(trimmed)
        ),
    }
}

fn truncate_command(cmd: &str) -> &str {
    let end = cmd.len().min(80);
    &cmd[..end]
}

fn is_test_path(path: &str) -> bool {
    // Match both relative and absolute paths containing tests/missouri/
    path.contains("tests/missouri/") || path.starts_with("tests/missouri")
}
