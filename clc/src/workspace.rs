//! `WorktreeWorkspace`: v1 implementation of the Workspace trait.
//!
//! Spawns Claude Code as a child process with piped stdio using stream-json
//! format. A background reader thread deserializes NDJSON from stdout and
//! pushes messages through an mpsc channel for non-blocking consumption.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use clc_sdk::stream::{InputMessage, OutputMessage, PermissionDenialMsg};
use clc_sdk::workspace::{
    PermissionDenial, Workspace, WorkspaceConfig, WorkspaceError, WorkspaceStatus,
};

pub struct WorktreeWorkspace {
    config: WorkspaceConfig,
    child: Option<Child>,
    stdin_handle: Option<std::process::ChildStdin>,
    rx: Option<mpsc::Receiver<OutputMessage>>,
    reader_handle: Option<thread::JoinHandle<()>>,
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
            child: None,
            stdin_handle: None,
            rx: None,
            reader_handle: None,
            status: WorkspaceStatus::NotStarted,
            denials: Vec::new(),
            worktree_dir,
        }
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new("claude");
        cmd.current_dir(&self.worktree_dir);

        cmd.arg("--print");
        cmd.arg("--input-format").arg("stream-json");
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--verbose");
        cmd.arg("--dangerously-skip-permissions");

        if let Some(ref model) = self.config.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(budget) = self.config.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(budget.to_string());
        }
        if let Some(ref sys) = self.config.system_prompt {
            cmd.arg("--append-system-prompt").arg(sys);
        }

        // Prompt is a positional argument, not --prompt.
        cmd.arg(&self.config.initial_prompt);

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        // Clear CLAUDECODE env var so the child doesn't think it's nested.
        cmd.env_remove("CLAUDECODE");

        cmd
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

        let mut cmd = self.build_command();
        let mut child = cmd
            .spawn()
            .map_err(|e| WorkspaceError::Process(format!("failed to spawn claude: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| WorkspaceError::Process("no stdin handle".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WorkspaceError::Process("no stdout handle".into()))?;

        let (tx, rx) = mpsc::channel();

        let reader_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(msg) = serde_json::from_str::<OutputMessage>(&line)
                    && tx.send(msg).is_err()
                {
                    break;
                }
            }
        });

        self.child = Some(child);
        self.stdin_handle = Some(stdin);
        self.rx = Some(rx);
        self.reader_handle = Some(reader_handle);
        self.status = WorkspaceStatus::Running;

        Ok(())
    }

    fn send_message(&mut self, msg: &str) -> Result<(), WorkspaceError> {
        let stdin = self
            .stdin_handle
            .as_mut()
            .ok_or_else(|| WorkspaceError::Communication("no stdin handle".into()))?;
        let input = InputMessage::user(msg);
        let json = serde_json::to_string(&input)
            .map_err(|e| WorkspaceError::Communication(format!("serialize: {e}")))?;
        writeln!(stdin, "{json}")
            .map_err(|e| WorkspaceError::Communication(format!("write: {e}")))?;
        stdin
            .flush()
            .map_err(|e| WorkspaceError::Communication(format!("flush: {e}")))?;
        Ok(())
    }

    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError> {
        let rx = self
            .rx
            .as_ref()
            .ok_or_else(|| WorkspaceError::Communication("not started".into()))?;

        let mut messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
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

        // Check if child has exited unexpectedly (no result message).
        if self.status == WorkspaceStatus::Running
            && let Some(ref mut child) = self.child
            && let Ok(Some(exit)) = child.try_wait()
        {
            self.status = if exit.success() {
                WorkspaceStatus::Completed
            } else {
                WorkspaceStatus::Failed
            };
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
        // Close stdin to signal EOF to the child.
        self.stdin_handle.take();

        // Wait for the child to exit.
        if let Some(ref mut child) = self.child {
            let _ = child.wait();
        }

        // Wait for the reader thread to finish.
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }
}
