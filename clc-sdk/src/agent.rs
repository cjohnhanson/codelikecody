//! Agent trait: an abstraction over the coding agent that runs inside a workspace.
//!
//! The workspace provides the isolated environment (worktree, container, etc.)
//! and handles IO wiring (stdin/stdout/stderr). The Agent builds the command
//! to execute. This decouples workspace management from agent-specific details.
//!
//! `ClaudeCodeAgent` is the first implementation — it builds a `claude` command
//! with stream-json IO. Future agents (different models, custom tools) implement
//! the same trait.

use std::path::Path;
use std::process::Command;

/// Configuration for starting an agent session.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model to use (e.g. "sonnet", "opus").
    pub model: String,
    /// System prompt appended to the agent's context.
    pub system_prompt: String,
    /// Initial user prompt that starts the work.
    pub initial_prompt: String,
    /// Extra CLI arguments passed to the agent binary.
    pub extra_args: Vec<String>,
    /// Tools the agent is allowed to use without permission prompts.
    /// Empty means no pre-approved tools (default permission mode applies).
    pub allowed_tools: Vec<String>,
}

/// Error from agent operations.
#[derive(Debug)]
pub enum AgentError {
    /// The agent binary was not found or could not be started.
    NotFound(String),
    /// Configuration error.
    Config(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "agent not found: {msg}"),
            Self::Config(msg) => write!(f, "agent config error: {msg}"),
        }
    }
}

impl std::error::Error for AgentError {}

/// A coding agent that can run inside a workspace.
///
/// The agent builds a `Command` configured for the workspace to spawn.
/// The workspace handles IO wiring (stdin, stdout, stderr) and process
/// lifecycle. The agent only knows how to build the right command.
pub trait Agent: std::fmt::Debug + Send {
    /// Build a command to start a new agent session.
    ///
    /// The returned command has the binary and arguments set but does NOT
    /// have stdin/stdout/stderr configured — the workspace sets those up.
    /// The command's `current_dir` is set to `working_dir`.
    fn build_start_command(
        &self,
        config: &AgentConfig,
        working_dir: &Path,
    ) -> Result<Command, AgentError>;

    /// Build a command to resume an existing agent session.
    ///
    /// `session_id` identifies the session to resume. The workspace
    /// provides the continuation prompt via stdin after spawning.
    fn build_resume_command(
        &self,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Command, AgentError>;

    /// The name of this agent type (for logging and config).
    fn name(&self) -> &str;
}

/// Claude Code agent implementation.
///
/// Builds `claude` commands with `--print --input-format stream-json
/// --output-format stream-json` for non-interactive operation.
#[derive(Debug)]
pub struct ClaudeCodeAgent {
    /// Path to the claude binary. Defaults to "claude" (found via PATH).
    binary: String,
}

impl ClaudeCodeAgent {
    /// Create a new Claude Code agent using the default binary name.
    #[must_use]
    pub fn new() -> Self {
        Self {
            binary: "claude".to_string(),
        }
    }

    /// Create a Claude Code agent with a specific binary path.
    #[must_use]
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for ClaudeCodeAgent {
    fn build_start_command(
        &self,
        config: &AgentConfig,
        working_dir: &Path,
    ) -> Result<Command, AgentError> {
        let mut cmd = Command::new(&self.binary);
        cmd.current_dir(working_dir);
        cmd.arg("--print");
        cmd.arg("--verbose");
        cmd.arg("--input-format").arg("stream-json");
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--model").arg(&config.model);
        cmd.arg("--append-system-prompt")
            .arg(&config.system_prompt);

        if !config.allowed_tools.is_empty() {
            cmd.arg("--allowedTools");
            cmd.arg(config.allowed_tools.join(" "));
        }

        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Prevent nested agent detection.
        cmd.env_remove("CLAUDECODE");

        Ok(cmd)
    }

    fn build_resume_command(
        &self,
        session_id: &str,
        working_dir: &Path,
    ) -> Result<Command, AgentError> {
        let mut cmd = Command::new(&self.binary);
        cmd.current_dir(working_dir);
        cmd.arg("--print");
        cmd.arg("--verbose");
        cmd.arg("--input-format").arg("stream-json");
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--resume").arg(session_id);

        cmd.env_remove("CLAUDECODE");

        Ok(cmd)
    }

    fn name(&self) -> &str {
        "claude-code"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn claude_code_start_command_has_required_args() {
        let agent = ClaudeCodeAgent::new();
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "test prompt".into(),
            initial_prompt: "do the thing".into(),
            extra_args: vec![],
            allowed_tools: vec![],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp/work"))
            .unwrap();

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.contains(&"--print".into()));
        assert!(args.contains(&"--input-format".into()));
        assert!(args.contains(&"stream-json".into()));
        assert!(args.contains(&"--output-format".into()));
        assert!(args.contains(&"--model".into()));
        assert!(args.contains(&"sonnet".into()));
        assert!(args.contains(&"--append-system-prompt".into()));
        assert!(args.contains(&"test prompt".into()));
    }

    #[test]
    fn claude_code_start_command_includes_extra_args() {
        let agent = ClaudeCodeAgent::new();
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "".into(),
            initial_prompt: "".into(),
            extra_args: vec!["--max-turns".into(), "5".into()],
            allowed_tools: vec![],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp"))
            .unwrap();

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.contains(&"--max-turns".into()));
        assert!(args.contains(&"5".into()));
    }

    #[test]
    fn claude_code_resume_command_has_session_id() {
        let agent = ClaudeCodeAgent::new();
        let cmd = agent
            .build_resume_command("test-session-123", &PathBuf::from("/tmp"))
            .unwrap();

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.contains(&"--resume".into()));
        assert!(args.contains(&"test-session-123".into()));
        assert!(args.contains(&"--print".into()));
        assert!(args.contains(&"stream-json".into()));
    }

    #[test]
    fn claude_code_removes_claudecode_env() {
        let agent = ClaudeCodeAgent::new();
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "".into(),
            initial_prompt: "".into(),
            extra_args: vec![],
            allowed_tools: vec![],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp"))
            .unwrap();

        let removals: Vec<_> = cmd
            .get_envs()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| k.to_string_lossy().to_string())
            .collect();
        assert!(removals.contains(&"CLAUDECODE".to_string()));
    }

    #[test]
    fn claude_code_custom_binary() {
        let agent = ClaudeCodeAgent::with_binary("/usr/local/bin/claude-custom");
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "".into(),
            initial_prompt: "".into(),
            extra_args: vec![],
            allowed_tools: vec![],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp"))
            .unwrap();

        assert_eq!(
            cmd.get_program().to_string_lossy(),
            "/usr/local/bin/claude-custom"
        );
    }

    #[test]
    fn claude_code_allowed_tools_emitted() {
        let agent = ClaudeCodeAgent::new();
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "".into(),
            initial_prompt: "".into(),
            extra_args: vec![],
            allowed_tools: vec!["Read".into(), "Bash(missouri agent*)".into()],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp"))
            .unwrap();

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(args.contains(&"--allowedTools".into()));
        assert!(args.contains(&"Read Bash(missouri agent*)".into()));
    }

    #[test]
    fn claude_code_no_allowed_tools_when_empty() {
        let agent = ClaudeCodeAgent::new();
        let config = AgentConfig {
            model: "sonnet".into(),
            system_prompt: "".into(),
            initial_prompt: "".into(),
            extra_args: vec![],
            allowed_tools: vec![],
        };
        let cmd = agent
            .build_start_command(&config, &PathBuf::from("/tmp"))
            .unwrap();

        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert!(!args.contains(&"--allowedTools".into()));
    }

    #[test]
    fn claude_code_name() {
        let agent = ClaudeCodeAgent::new();
        assert_eq!(agent.name(), "claude-code");
    }
}
