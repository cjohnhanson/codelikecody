//! `WorktreeWorkspace`: v1 implementation of the Workspace trait.
//!
//! Uses the pipe-based dispatch infrastructure (named FIFOs, stdout.jsonl, pid file)
//! so that the coordinator process is detached and observable via
//! `clc coordinator check/log/send` after `clc coordinate` exits.

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use claude_code::protocol::{OutputMessage, PermissionDenialMsg};
use clc_sdk::workspace::{
    PermissionDenial, Workspace, WorkspaceConfig, WorkspaceError, WorkspaceStatus,
};
use nix::sys::signal;
use nix::unistd::Pid;

use crate::dispatch;
use crate::worker;

pub struct WorktreeWorkspace {
    config: WorkspaceConfig,
    working_dir: PathBuf,
    worker_dir: PathBuf,
    status: WorkspaceStatus,
    denials: Vec<PermissionDenial>,
    /// Read cursor position in stdout.jsonl for incremental polling.
    stdout_cursor: u64,
    pid: Option<u32>,
}

impl WorktreeWorkspace {
    #[must_use]
    pub fn new(config: WorkspaceConfig) -> Self {
        // Use the same path logic as worker::working_dir_for / worker_dir_for so
        // the coordinator (COORDINATOR_ID) resolves to trunk and regular workers
        // resolve to their worktree.
        let working_dir = worker::working_dir_for(&config.project_dir, &config.tisket_id);
        let worker_dir = worker::worker_dir_for(&config.project_dir, &config.tisket_id);
        Self {
            config,
            working_dir,
            worker_dir,
            status: WorkspaceStatus::NotStarted,
            denials: Vec::new(),
            stdout_cursor: 0,
            pid: None,
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

    fn pid_alive(&self) -> bool {
        self.pid
            .is_some_and(|pid| signal::kill(Pid::from_raw(pid.cast_signed()), None).is_ok())
    }
}

impl Workspace for WorktreeWorkspace {
    fn start(&mut self) -> Result<(), WorkspaceError> {
        if self.status != WorkspaceStatus::NotStarted {
            return Err(WorkspaceError::Process("workspace already started".into()));
        }

        let model = self.config.model.as_deref().unwrap_or("claude-sonnet-4-6");
        let system_prompt = self.config.system_prompt.as_deref().unwrap_or("");

        let pid = dispatch::spawn_worker_process(
            &self.working_dir,
            &self.worker_dir,
            model,
            system_prompt,
            &self.config.initial_prompt,
            &[],
        )
        .map_err(|e| WorkspaceError::Process(format!("{e}")))?;

        self.pid = Some(pid);
        self.status = WorkspaceStatus::Running;
        Ok(())
    }

    fn send_message(&mut self, msg: &str) -> Result<(), WorkspaceError> {
        let pipe_path = self.worker_dir.join("stdin.pipe");
        dispatch::send_prompt(&pipe_path, msg)
            .map_err(|e| WorkspaceError::Communication(format!("{e}")))
    }

    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError> {
        if self.status == WorkspaceStatus::NotStarted {
            return Err(WorkspaceError::Communication("not started".into()));
        }

        let stdout_path = self.worker_dir.join("stdout.jsonl");
        let mut messages = Vec::new();

        if stdout_path.exists() {
            let mut file = std::fs::File::open(&stdout_path)
                .map_err(|e| WorkspaceError::Communication(format!("open stdout.jsonl: {e}")))?;
            file.seek(std::io::SeekFrom::Start(self.stdout_cursor))
                .map_err(|e| WorkspaceError::Communication(format!("seek stdout.jsonl: {e}")))?;

            let mut content = String::new();
            file.read_to_string(&mut content)
                .map_err(|e| WorkspaceError::Communication(format!("read stdout.jsonl: {e}")))?;

            #[allow(clippy::cast_possible_truncation)]
            {
                self.stdout_cursor += content.len() as u64;
            }

            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(msg) = serde_json::from_str::<OutputMessage>(line) {
                    if let OutputMessage::Result(ref result) = msg {
                        self.denials = Self::denials_from_msg(&result.permission_denials);
                        self.status = if result.is_error {
                            WorkspaceStatus::Failed
                        } else {
                            WorkspaceStatus::Completed
                        };
                    }
                    messages.push(msg);
                }
            }
        }

        // Check if the process exited without sending a result message.
        if self.status == WorkspaceStatus::Running && !self.pid_alive() {
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
        &self.working_dir
    }

    fn tisket_id(&self) -> &str {
        &self.config.tisket_id
    }

    fn stop(&mut self) -> Result<(), WorkspaceError> {
        if let Some(pid) = self.pid {
            let _ = signal::kill(Pid::from_raw(pid.cast_signed()), signal::Signal::SIGTERM);
        }
        Ok(())
    }
}
