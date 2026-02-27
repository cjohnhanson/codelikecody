//! `WorktreeWorkspace`: v1 implementation of the Workspace trait.
//!
//! Delegates process management to `claude_code::Session` and adds
//! clc-specific bookkeeping (status tracking, permission denial extraction).

use std::path::{Path, PathBuf};

use claude_code::protocol::{OutputMessage, PermissionDenialMsg};
use claude_code::session::{Session, SessionConfig};
use clc_sdk::workspace::{
    PermissionDenial, Workspace, WorkspaceConfig, WorkspaceError, WorkspaceStatus,
};

pub struct WorktreeWorkspace {
    config: WorkspaceConfig,
    session: Option<Session>,
    status: WorkspaceStatus,
    denials: Vec<PermissionDenial>,
    worktree_dir: PathBuf,
}

impl WorktreeWorkspace {
    #[must_use]
    pub fn new(config: WorkspaceConfig) -> Self {
        let worktree_dir = config
            .project_dir
            .join(".worktrees")
            .join(&config.tisket_id);
        Self {
            config,
            session: None,
            status: WorkspaceStatus::NotStarted,
            denials: Vec::new(),
            worktree_dir,
        }
    }

    fn denials_from_msg(denials: &[PermissionDenialMsg]) -> Vec<PermissionDenial> {
        denials
            .iter()
            .map(|d| PermissionDenial {
                tool_name: d.tool_name.clone(),
                message: d.message.clone(),
            })
            .collect()
    }
}

impl Workspace for WorktreeWorkspace {
    fn start(&mut self) -> Result<(), WorkspaceError> {
        if self.status != WorkspaceStatus::NotStarted {
            return Err(WorkspaceError::Process("workspace already started".into()));
        }

        let session_config = SessionConfig {
            working_dir: self.worktree_dir.clone(),
            initial_prompt: self.config.initial_prompt.clone(),
            model: self.config.model.clone(),
            max_budget_usd: self.config.max_budget_usd,
            system_prompt: self.config.system_prompt.clone(),
            verbose: true,
            dangerously_skip_permissions: true,
        };

        let session =
            Session::start(&session_config).map_err(|e| WorkspaceError::Process(format!("{e}")))?;

        self.session = Some(session);
        self.status = WorkspaceStatus::Running;
        Ok(())
    }

    fn send_message(&mut self, msg: &str) -> Result<(), WorkspaceError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| WorkspaceError::Communication("not started".into()))?;
        session
            .send(msg)
            .map_err(|e| WorkspaceError::Communication(format!("{e}")))
    }

    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| WorkspaceError::Communication("not started".into()))?;

        let messages = session.recv();

        for msg in &messages {
            if let OutputMessage::Result(result) = msg {
                self.denials = Self::denials_from_msg(&result.permission_denials);
                self.status = if result.is_error {
                    WorkspaceStatus::Failed
                } else {
                    WorkspaceStatus::Completed
                };
            }
        }

        // Check if child has exited unexpectedly (no result message).
        if self.status == WorkspaceStatus::Running && !session.is_running() {
            self.status = WorkspaceStatus::Failed;
        }

        Ok(messages)
    }

    fn status(&self) -> WorkspaceStatus {
        self.status
    }

    fn permission_denials(&self) -> &[PermissionDenial] {
        &self.denials
    }

    fn working_dir(&self) -> &Path {
        &self.worktree_dir
    }

    fn tisket_id(&self) -> &str {
        &self.config.tisket_id
    }

    fn stop(&mut self) -> Result<(), WorkspaceError> {
        if let Some(ref mut session) = self.session {
            session.stop();
        }
        Ok(())
    }
}
