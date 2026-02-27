//! Workspace trait: an isolated environment where an agent works,
//! plus a control channel for communicating with it.
//!
//! v1 implementation is a git worktree + Claude Code child process with
//! stream-json. Future backends (Docker, Coder, K8s) swap in without
//! changing the coordinator loop.

use std::path::{Path, PathBuf};

use crate::protocol::OutputMessage;

/// Status of a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceStatus {
    /// Worker process has not been started yet.
    NotStarted,
    /// Worker process is running.
    Running,
    /// Worker completed successfully (result message received).
    Completed,
    /// Worker exited with an error or non-zero status.
    Failed,
}

/// A tool use that was denied by the permission system.
#[derive(Debug, Clone)]
pub struct PermissionDenial {
    pub tool_name: String,
    pub message: String,
}

/// Workspace error.
#[derive(Debug)]
pub enum WorkspaceError {
    /// Workspace creation or startup failed.
    Creation(String),
    /// Communication with the worker failed (stdin/stdout).
    Communication(String),
    /// Worker process error (spawn, wait, unexpected exit).
    Process(String),
    /// Workspace teardown failed.
    Teardown(String),
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creation(msg) => write!(f, "workspace creation: {msg}"),
            Self::Communication(msg) => write!(f, "workspace communication: {msg}"),
            Self::Process(msg) => write!(f, "workspace process: {msg}"),
            Self::Teardown(msg) => write!(f, "workspace teardown: {msg}"),
        }
    }
}

impl std::error::Error for WorkspaceError {}

/// Configuration for creating a workspace.
pub struct WorkspaceConfig {
    pub tisket_id: String,
    pub project_dir: PathBuf,
    pub main_branch: String,
    pub initial_prompt: String,
    pub system_prompt: Option<String>,
    pub max_budget_usd: Option<f64>,
    pub model: Option<String>,
}

/// An isolated environment where an agent works, plus a control channel.
///
/// The coordinator creates workspaces, monitors them via `recv_output()`,
/// and calls `stop()` when done. Worktree/branch cleanup is handled
/// separately by the coordinator (via `gix_ops` functions), not by the
/// workspace itself.
pub trait Workspace {
    /// Launch the worker agent in this workspace.
    fn start(&mut self) -> Result<(), WorkspaceError>;

    /// Send a follow-up message to the worker via stdin.
    fn send_message(&mut self, msg: &str) -> Result<(), WorkspaceError>;

    /// Drain buffered output messages from the worker.
    /// Non-blocking: returns whatever has been received since the last call.
    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError>;

    /// Current status of the workspace.
    fn status(&self) -> WorkspaceStatus;

    /// Permission denials from the most recent result message.
    fn permission_denials(&self) -> &[PermissionDenial];

    /// The working directory of this workspace.
    fn working_dir(&self) -> &Path;

    /// The tisket ID this workspace is working on.
    fn tisket_id(&self) -> &str;

    /// Stop the worker process. Does NOT remove the worktree or branch.
    fn stop(&mut self) -> Result<(), WorkspaceError>;
}
