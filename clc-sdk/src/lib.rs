pub mod stream;
pub mod workspace;

/// Returns `true` when running inside a coding agent (Claude Code sets
/// `CLAUDECODE=1` in the shell environment).
#[must_use]
pub fn in_agent_context() -> bool {
    std::env::var("CLAUDECODE").is_ok()
}

/// Context passed to `ClcTool::prime()` so each tool can adapt its output
/// to the current workflow state.
pub struct PrimeContext {
    /// Current workflow phase, if set (e.g. "tests-unwritten", "implementing").
    pub phase: Option<String>,
}

/// Shared interface implemented by each tool in the codelikecody ecosystem.
///
/// clc calls these methods at the appropriate points in the agent lifecycle
/// (`SessionStart`, periodic reinforcement, etc.) based on the current hook
/// event and workflow state.
pub trait ClcTool {
    /// Imperative directives for agents. Content adapts to the current phase —
    /// richer detail when the agent needs to learn (e.g. test authoring in
    /// `tests-unwritten`), leaner when it just needs a nudge.
    fn prime(&self, ctx: &PrimeContext) -> String;

    /// One-liner summary for periodic context reinforcement.
    fn status_basic(&self) -> String;

    /// Complete state dump for session start on feature branches.
    fn status_full(&self) -> String;
}
