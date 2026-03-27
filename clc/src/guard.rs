use std::path::Path;

use serde_json::Value;

use crate::config::PermissionsDef;
use crate::event::{Event, Response};
use crate::git::GitState;
use crate::git_add;
use crate::phase::Phase;
use crate::workflow::Workflow;

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

/// Evaluate an event against the current git state and phase (legacy, uses Phase enum).
pub fn evaluate(event: &Event, git: Option<&GitState>, phase: Option<Phase>, cwd: &Path) -> Response {
    // Escape hatch: set CLC_GUARD_OFF=1 to bypass all guard checks.
    // Used during clc development when the guard itself is being modified.
    if std::env::var("CLC_GUARD_OFF").is_ok_and(|v| !v.is_empty()) {
        return Response::Passthrough;
    }
    match event {
        Event::PreToolUse {
            tool_name,
            tool_input,
        } => check_tool_use(tool_name, tool_input, git, phase, cwd),
        Event::Stop => check_stop(git, phase),
        _ => Response::Passthrough,
    }
}

/// Evaluate an event using the workflow engine for phase enforcement.
#[allow(dead_code)] // Consumed by hook migration in upcoming commit
pub fn evaluate_with_workflow(
    event: &Event,
    git: Option<&GitState>,
    phase_name: Option<&str>,
    workflow: &Workflow,
    cwd: &Path,
) -> Response {
    if std::env::var("CLC_GUARD_OFF").is_ok_and(|v| !v.is_empty()) {
        return Response::Passthrough;
    }
    match event {
        Event::PreToolUse {
            tool_name,
            tool_input,
        } => check_tool_use_workflow(tool_name, tool_input, git, phase_name, workflow, cwd),
        Event::Stop => check_stop_workflow(git, phase_name, workflow),
        _ => Response::Passthrough,
    }
}

fn check_stop_workflow(
    git: Option<&GitState>,
    phase_name: Option<&str>,
    workflow: &Workflow,
) -> Response {
    let Some(state) = git else {
        return Response::Passthrough;
    };

    if state.is_main || state.is_admin {
        return Response::Passthrough;
    }

    match phase_name {
        None => Response::Block {
            message: "No phase set — work has not started. \
                      Set a phase or run `clc pickup`."
                .to_string(),
        },
        Some(name) if workflow.can_stop(name) => Response::Passthrough,
        Some(name) => Response::Block {
            message: format!(
                "Work is not complete. Current phase: {name}. \
                 Run `clc done` to finalize."
            ),
        },
    }
}

fn check_tool_use_workflow(
    tool_name: &str,
    tool_input: &Value,
    git: Option<&GitState>,
    phase_name: Option<&str>,
    workflow: &Workflow,
    cwd: &Path,
) -> Response {
    // Git-add validation runs on all branches, before any other checks.
    if tool_name == "Bash" {
        if let Some(command) = tool_input.get("command").and_then(Value::as_str) {
            if let git_add::GitAddCheck::Blocked(reason) = git_add::validate(command, cwd) {
                return Response::Block { message: reason };
            }
        }
    }

    let Some(state) = git else {
        return Response::Passthrough;
    };

    // Main branch guard: unchanged — project-level, not workflow.
    if state.is_main {
        if READ_ONLY_TOOLS.contains(&tool_name) {
            return Response::Passthrough;
        }

        if tool_name == "Bash" {
            return check_bash_allowlist(tool_input);
        }

        return Response::Block {
            message: format!(
                "Blocked: {tool_name} is not allowed on trunk.\n\
                 File modifications are not permitted on the main branch.\n\
                 Pick up a tisket to begin work: `clc pickup <issue-id>`"
            ),
        };
    }

    // Admin branch: fully permissive.
    if state.is_admin {
        return Response::Passthrough;
    }

    // Feature branch: read-only tools always pass.
    if READ_ONLY_TOOLS.contains(&tool_name) {
        return Response::Passthrough;
    }

    // Phase enforcement via workflow permissions.
    let current = phase_name.unwrap_or(workflow.initial_phase());

    match workflow.phase_permissions(current) {
        None => Response::Passthrough, // No permissions = unrestricted
        Some(perms) => check_permissions(tool_name, tool_input, current, perms),
    }
}

/// Evaluate a tool call against a phase's permission rules.
/// Deny rules are checked first; allow rules punch exceptions.
fn check_permissions(
    tool_name: &str,
    tool_input: &Value,
    phase: &str,
    perms: &PermissionsDef,
) -> Response {
    // If no deny rules, everything is allowed.
    if perms.deny.is_empty() {
        return Response::Passthrough;
    }

    // Check if the tool is denied.
    let denied = perms.deny.iter().any(|pattern| tool_matches(tool_name, tool_input, pattern));

    if !denied {
        return Response::Passthrough;
    }

    // Tool is denied — check if an allow rule grants an exception.
    let allowed = perms
        .allow
        .iter()
        .any(|pattern| tool_matches(tool_name, tool_input, pattern));

    if allowed {
        return Response::Passthrough;
    }

    // Build a helpful error message.
    let file_path = tool_input
        .get("file_path")
        .and_then(Value::as_str)
        .or_else(|| tool_input.get("command").and_then(Value::as_str).map(|c| truncate_command(c)));

    let target_info = file_path.map_or(String::new(), |p| format!(" targeting '{p}'"));

    Response::Block {
        message: format!(
            "Blocked: {tool_name}{target_info} is not allowed in phase '{phase}'.\n\
             Phase permissions restrict this action."
        ),
    }
}

/// Match a tool call against a permission pattern.
/// Pattern formats:
/// - `"Edit"` — matches the tool name exactly
/// - `"Edit(tests/**)"` — matches the tool name AND the file_path/command against the glob
/// - `"Bash(cargo test *)"` — matches Bash AND the command against the glob
fn tool_matches(tool_name: &str, tool_input: &Value, pattern: &str) -> bool {
    if let Some(paren_start) = pattern.find('(') {
        // Pattern with glob: "Tool(glob)"
        let pat_tool = &pattern[..paren_start];
        if pat_tool != tool_name {
            return false;
        }
        let glob = &pattern[paren_start + 1..pattern.len().saturating_sub(1)];

        let value = if tool_name == "Bash" {
            tool_input.get("command").and_then(Value::as_str).unwrap_or("")
        } else {
            tool_input
                .get("file_path")
                .and_then(Value::as_str)
                .unwrap_or("")
        };

        glob_match(glob, value)
    } else {
        // Bare tool name match.
        pattern == tool_name
    }
}

/// Simple glob matching: `*` matches any sequence within a path component,
/// `**` matches any sequence including path separators.
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "**" || pattern == "*" {
        return true;
    }

    // Convert glob to a simple regex-like check.
    // Split pattern on ** first, then handle * within segments.
    let parts: Vec<&str> = pattern.split("**").collect();
    if parts.len() == 1 {
        // No ** — just handle * as "anything except /"
        return simple_glob_match(pattern, value);
    }

    // ** matching: each part must appear in order in value
    let mut remaining = value;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First part must match at the start
            if !simple_glob_prefix(part, remaining) {
                return false;
            }
            remaining = &remaining[simple_glob_match_len(part, remaining)..];
        } else if let Some(pos) = find_glob_match(part, remaining) {
            remaining = &remaining[pos..];
        } else {
            return false;
        }
    }
    true
}

/// Match a simple glob (with * but no **) against a full string.
fn simple_glob_match(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == value;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !value.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if let Some(found) = value[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

fn simple_glob_prefix(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    value.starts_with(parts[0])
}

fn simple_glob_match_len(pattern: &str, value: &str) -> usize {
    // Approximate: return the length consumed by the pattern match
    let parts: Vec<&str> = pattern.split('*').collect();
    let literal_len: usize = parts.iter().map(|p| p.len()).sum();
    literal_len.min(value.len())
}

fn find_glob_match(pattern: &str, value: &str) -> Option<usize> {
    let first_literal = pattern.split('*').next().unwrap_or("");
    if first_literal.is_empty() {
        return Some(0);
    }
    value.find(first_literal).map(|pos| pos + first_literal.len())
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
    cwd: &Path,
) -> Response {
    // Git-add validation runs on all branches, before any other checks.
    if tool_name == "Bash" {
        if let Some(command) = tool_input.get("command").and_then(Value::as_str) {
            if let git_add::GitAddCheck::Blocked(reason) = git_add::validate(command, cwd) {
                return Response::Block { message: reason };
            }
        }
    }

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

    fn test_cwd() -> &'static Path {
        Path::new("/tmp")
    }

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
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::ReviewRequested), test_cwd());
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Stop hook: reviewed allows exit ---

    #[test]
    fn stop_allowed_at_reviewed() {
        let git = feature_branch();
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::Reviewed), test_cwd());
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
        let resp = evaluate(&stop_event(), Some(&git), Some(Phase::Done), test_cwd());
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
        let resp = evaluate(&edit_src_event(), Some(&git), Some(Phase::InReview), test_cwd());
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
            test_cwd(),
        );
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn edit_test_allowed_in_review_requested() {
        let git = feature_branch();
        let resp = evaluate(&edit_test_event(), Some(&git), Some(Phase::ReviewRequested), test_cwd());
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
            test_cwd(),
        );
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn edit_test_allowed_in_reviewed() {
        let git = feature_branch();
        let resp = evaluate(&edit_test_event(), Some(&git), Some(Phase::Reviewed), test_cwd());
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
            test_cwd(),
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
            test_cwd(),
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Workflow-based guard tests ---

    fn tdd_workflow() -> Workflow {
        Workflow::default_tdd()
    }

    #[test]
    fn workflow_stop_allowed_at_can_stop_phase() {
        let git = feature_branch();
        let wf = tdd_workflow();
        // green has can_stop: true
        let resp = check_stop_workflow(Some(&git), Some("green"), &wf);
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn workflow_stop_allowed_at_terminal() {
        let git = feature_branch();
        let wf = tdd_workflow();
        let resp = check_stop_workflow(Some(&git), Some("done"), &wf);
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn workflow_stop_blocked_at_non_stop_phase() {
        let git = feature_branch();
        let wf = tdd_workflow();
        let resp = check_stop_workflow(Some(&git), Some("implementing"), &wf);
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn workflow_stop_blocked_when_no_phase() {
        let git = feature_branch();
        let wf = tdd_workflow();
        let resp = check_stop_workflow(Some(&git), None, &wf);
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn workflow_unrestricted_phase_allows_edit() {
        let git = feature_branch();
        let wf = tdd_workflow();
        // implementing has no permissions (unrestricted)
        let resp = check_tool_use_workflow(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            Some("implementing"),
            &wf,
            test_cwd(),
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn workflow_restricted_phase_blocks_src_edit() {
        let git = feature_branch();
        let wf = tdd_workflow();
        // tests-unwritten has permissions: deny Edit, allow Edit(tests/**)
        let resp = check_tool_use_workflow(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            Some("tests-unwritten"),
            &wf,
            test_cwd(),
        );
        assert!(matches!(resp, Response::Block { .. }));
    }

    #[test]
    fn workflow_restricted_phase_allows_test_edit() {
        let git = feature_branch();
        let wf = tdd_workflow();
        let resp = check_tool_use_workflow(
            "Edit",
            &json!({"file_path": "tests/missouri/foo/.missouri/missouri.yml"}),
            Some(&git),
            Some("tests-unwritten"),
            &wf,
            test_cwd(),
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn workflow_read_only_always_passes() {
        let git = feature_branch();
        let wf = tdd_workflow();
        let resp = check_tool_use_workflow(
            "Read",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            Some("tests-unwritten"),
            &wf,
            test_cwd(),
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn workflow_admin_branch_unrestricted() {
        let git = admin_branch();
        let wf = tdd_workflow();
        let resp = check_tool_use_workflow(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            Some(&git),
            None,
            &wf,
            test_cwd(),
        );
        assert!(matches!(resp, Response::Passthrough));
    }

    // --- Permission matching tests ---

    #[test]
    fn tool_matches_bare_name() {
        assert!(tool_matches("Edit", &json!({}), "Edit"));
        assert!(!tool_matches("Write", &json!({}), "Edit"));
    }

    #[test]
    fn tool_matches_glob_file_path() {
        assert!(tool_matches(
            "Edit",
            &json!({"file_path": "tests/missouri/foo.yml"}),
            "Edit(tests/**)"
        ));
        assert!(!tool_matches(
            "Edit",
            &json!({"file_path": "src/main.rs"}),
            "Edit(tests/**)"
        ));
    }

    #[test]
    fn tool_matches_bash_command_glob() {
        assert!(tool_matches(
            "Bash",
            &json!({"command": "cargo test --workspace"}),
            "Bash(cargo test *)"
        ));
        assert!(!tool_matches(
            "Bash",
            &json!({"command": "rm -rf /"}),
            "Bash(cargo test *)"
        ));
    }

    #[test]
    fn check_permissions_deny_then_allow() {
        let perms = PermissionsDef {
            deny: vec!["Edit".into()],
            allow: vec!["Edit(tests/**)".into()],
        };
        // Denied: src edit
        let resp = check_permissions("Edit", &json!({"file_path": "src/main.rs"}), "test-phase", &perms);
        assert!(matches!(resp, Response::Block { .. }));
        // Allowed: test edit (allow punches exception)
        let resp = check_permissions("Edit", &json!({"file_path": "tests/foo.rs"}), "test-phase", &perms);
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn check_permissions_no_deny_allows_all() {
        let perms = PermissionsDef {
            deny: vec![],
            allow: vec!["Edit(tests/**)".into()],
        };
        let resp = check_permissions("Edit", &json!({"file_path": "src/main.rs"}), "test-phase", &perms);
        assert!(matches!(resp, Response::Passthrough));
    }

    #[test]
    fn check_permissions_not_denied_tool_passes() {
        let perms = PermissionsDef {
            deny: vec!["Edit".into()],
            allow: vec![],
        };
        // Bash is not in the deny list
        let resp = check_permissions("Bash", &json!({"command": "ls"}), "test-phase", &perms);
        assert!(matches!(resp, Response::Passthrough));
    }
}
