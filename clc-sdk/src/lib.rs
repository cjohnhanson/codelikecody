/// Returns `true` when running inside a coding agent (Claude Code sets
/// `CLAUDECODE=1` in the shell environment).
#[must_use]
pub fn in_agent_context() -> bool {
    std::env::var("CLAUDECODE").is_ok()
}

/// Shared interface implemented by each tool in the codelikecody ecosystem.
///
/// clc calls these methods at the appropriate points in the agent lifecycle
/// (`SessionStart`, periodic reinforcement, etc.) based on the current hook
/// event and workflow state.
pub trait ClcTool {
    /// Imperative directives for agents. Asserts requirements — not offering
    /// information but commanding behavior.
    fn prime(&self) -> String;

    /// One-liner summary for periodic context reinforcement.
    fn status_basic(&self) -> String;

    /// Complete state dump for session start on feature branches.
    fn status_full(&self) -> String;
}
