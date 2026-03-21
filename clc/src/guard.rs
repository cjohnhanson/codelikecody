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
    "zettel note list",
    "zettel note show",
    "zettel backlinks",
    "zettel orphans",
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
    if std::env::var("CLC_GUARD_OFF").is_ok_and(|v| !v.is_empty()) {
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

    if state.is_main || state.is_admin {
        return Response::Passthrough;
    }

    let message = match phase {
        None => "No phase set — work has not started. \
                 Set a phase with `clc status set tests-unwritten` or run `clc pickup`."
            .to_string(),
        Some(Phase::Done | Phase::ReviewRequested | Phase::Reviewed) => {
            return Response::Passthrough;
        }
        Some(current_phase) => format!(
            "Work is not complete. Current phase: {current_phase}. \
             Run `clc done` to finalize."
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

    // Admin branch: fully permissive — no phase enforcement.
    if state.is_admin {
        return Response::Passthrough;
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
        Phase::Implementing | Phase::InReview => Response::Passthrough,
        Phase::TestsUnwritten
        | Phase::TestsWritten
        | Phase::Red
        | Phase::Green
        | Phase::ReviewRequested
        | Phase::Reviewed
        | Phase::Done => check_phase_restricted(tool_name, tool_input, current_phase),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn feature_branch() -> GitState {
        GitState {
            branch: "feat-xyz".to_string(),
            is_main: false,
            is_admin: false,
            is_worktree: true,
        }
    }

    fn stop_event() -> Event {
        Event::Stop
    }

    fn edit_src_event() -> Event {
        Event::PreToolUse {
            tool_name: "Edit".to_string(),
            tool_input: json!({"file_path": "src/main.rs"}),
        }
    }

    fn edit_test_event() -> Event {
        Event::PreToolUse {
            tool_name: "Edit".to_string(),
            tool_input: json!({"file_path": "tests/missouri/foo/.missouri/missouri.yml"}),
        }
    }

    // --- Stop hook: review-requested allows exit ---

    #[test]
    fn stop_allowed_at_review_requested() {
        let git = feature_branch();
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::ReviewRequested));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Stop hook: reviewed allows exit ---

    #[test]
    fn stop_allowed_at_reviewed() {
        let git = feature_branch();
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::Reviewed));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Stop hook: in-review blocks exit ---

    #[test]
    fn stop_blocked_at_in_review() {
        let git = feature_branch();
        let resp = check_stop(Some(&git), Some(Phase::InReview));
        assert!(matches!(resp, Response::Block { .. }));
    }

    // --- Stop hook: done still allows exit ---

    #[test]
    fn stop_allowed_at_done() {
        let git = feature_branch();
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::Done));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Stop hook: green still blocks exit ---

    #[test]
    fn stop_blocked_at_green() {
        let git = feature_branch();
        let resp = check_stop(Some(&git), Some(Phase::Green));
        assert!(matches!(resp, Response::Block { .. }));
    }

    // --- PreToolUse: in-review unrestricted (like implementing) ---

    #[test]
    fn edit_src_allowed_in_in_review() {
        let git = feature_branch();
        let resp = evaluate(&edit_src_event(), Some(&git), Some(Phase::InReview));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- PreToolUse: review-requested restricted (only test paths) ---

    #[test]
    fn edit_src_blocked_in_review_requested() {
        let git = feature_branch();
        let resp = check_tool_use(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            Some(Phase::ReviewRequested),
        );
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn edit_test_allowed_in_review_requested() {
        let git = feature_branch();
        let resp = evaluate(&edit_test_event(), Some(&git), Some(Phase::ReviewRequested));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- PreToolUse: reviewed restricted (only test paths) ---

    #[test]
    fn edit_src_blocked_in_reviewed() {
        let git = feature_branch();
        let resp = check_tool_use(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            Some(Phase::Reviewed),
        );
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn edit_test_allowed_in_reviewed() {
        let git = feature_branch();
        let resp = evaluate(&edit_test_event(), Some(&git), Some(Phase::Reviewed));
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Admin branch: fully permissive ---

    fn admin_branch() -> GitState {
        GitState {
            branch: "clc-admin".to_string(),
            is_main: false,
            is_admin: true,
            is_worktree: true,
        }
    }

    #[test]
    fn admin_edit_src_allowed_without_phase() {
        let git = admin_branch();
        // Call check_tool_use directly to bypass CLC_GUARD_OFF escape hatch.
        let resp = check_tool_use(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            None,
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn admin_stop_allowed_without_phase() {
        let git = admin_branch();
        // Call check_stop directly to bypass CLC_GUARD_OFF escape hatch.
        let resp = check_stop(Some(&git), None);
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn admin_bash_unrestricted() {
        let git = admin_branch();
        // Call check_tool_use directly to bypass CLC_GUARD_OFF escape hatch.
        let resp = check_tool_use(
            "Bash",
            &json!({"command": "rm -rf /tmp/junk"}),
            Some(&git),
            None,
        );
        assert!(matches!(resp, Response::Passthrough));
    }
}
