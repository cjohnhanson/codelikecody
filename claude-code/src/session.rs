//! Claude Code session: spawn and communicate with a Claude Code child process.
//!
//! Uses `--print --input-format stream-json --output-format stream-json` for
//! structured NDJSON over piped stdio. A background reader thread deserializes
//! output lines and pushes them through an mpsc channel for non-blocking
//! consumption.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::protocol::{InputMessage, OutputMessage};

/// Configuration for launching a Claude Code session.
pub struct SessionConfig {
    /// Working directory for the claude process.
    pub working_dir: PathBuf,
    /// Initial prompt (passed as positional argument).
    pub initial_prompt: String,
    /// Model to use (e.g. "opus", "sonnet").
    pub model: Option<String>,
    /// Maximum spend for this session.
    pub max_budget_usd: Option<f64>,
    /// System prompt appended via `--append-system-prompt`.
    pub system_prompt: Option<String>,
    /// Run with `--dangerously-skip-permissions`.
    pub dangerously_skip_permissions: bool,
}

/// Error from a Claude Code session.
#[derive(Debug)]
pub enum SessionError {
    /// Failed to spawn the claude process.
    Spawn(String),
    /// Failed to communicate with the process (stdin/stdout).
    Communication(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(msg) => write!(f, "session spawn: {msg}"),
            Self::Communication(msg) => write!(f, "session communication: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// A running Claude Code session with piped stdio.
pub struct Session {
    child: Child,
    stdin_handle: Option<std::process::ChildStdin>,
    rx: mpsc::Receiver<OutputMessage>,
    reader_handle: Option<thread::JoinHandle<()>>,
}

impl Session {
    /// Spawn a new Claude Code process and start reading its output.
    ///
    /// The initial prompt is sent as a stream-json user message on stdin
    /// immediately after spawning. This is required because Claude Code
    /// with `--input-format stream-json` ignores positional prompt arguments.
    pub fn start(config: &SessionConfig) -> Result<Self, SessionError> {
        let mut cmd = Self::build_command(config);
        let mut child = cmd
            .spawn()
            .map_err(|e| SessionError::Spawn(format!("failed to spawn claude: {e}")))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| SessionError::Spawn("no stdin handle".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SessionError::Spawn("no stdout handle".into()))?;

        // Send the initial prompt as a stream-json message.
        let initial = InputMessage::user(&config.initial_prompt);
        let json = serde_json::to_string(&initial)
            .map_err(|e| SessionError::Spawn(format!("serialize initial prompt: {e}")))?;
        writeln!(stdin, "{json}")
            .map_err(|e| SessionError::Spawn(format!("write initial prompt: {e}")))?;
        stdin
            .flush()
            .map_err(|e| SessionError::Spawn(format!("flush initial prompt: {e}")))?;

        let (tx, rx) = mpsc::channel();

        let reader_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<OutputMessage>(&line) {
                    Ok(msg) => {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[session reader] unparseable: {e}");
                        eprintln!("[session reader]   raw: {line}");
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin_handle: Some(stdin),
            rx,
            reader_handle: Some(reader_handle),
        })
    }

    /// Send a follow-up message to the Claude Code process via stdin.
    pub fn send(&mut self, msg: &str) -> Result<(), SessionError> {
        let stdin = self
            .stdin_handle
            .as_mut()
            .ok_or_else(|| SessionError::Communication("stdin closed".into()))?;
        let input = InputMessage::user(msg);
        let json = serde_json::to_string(&input)
            .map_err(|e| SessionError::Communication(format!("serialize: {e}")))?;
        writeln!(stdin, "{json}")
            .map_err(|e| SessionError::Communication(format!("write: {e}")))?;
        stdin
            .flush()
            .map_err(|e| SessionError::Communication(format!("flush: {e}")))?;
        Ok(())
    }

    /// Drain buffered output messages. Non-blocking: returns whatever has
    /// been received since the last call.
    #[must_use]
    pub fn recv(&self) -> Vec<OutputMessage> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Check whether the child process is still running.
    #[must_use]
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Stop the session: close stdin, wait for exit, join reader thread.
    pub fn stop(&mut self) {
        // Close stdin to signal EOF.
        self.stdin_handle.take();

        // Wait for the child to exit.
        let _ = self.child.wait();

        // Wait for the reader thread to finish.
        if let Some(handle) = self.reader_handle.take() {
            let _ = handle.join();
        }
    }

    fn build_command(config: &SessionConfig) -> Command {
        let mut cmd = Command::new("claude");
        cmd.current_dir(&config.working_dir);

        cmd.arg("--print");
        cmd.arg("--verbose");
        cmd.arg("--input-format").arg("stream-json");
        cmd.arg("--output-format").arg("stream-json");
        if config.dangerously_skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }
        if let Some(ref model) = config.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(budget) = config.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(budget.to_string());
        }
        if let Some(ref sys) = config.system_prompt {
            cmd.arg("--append-system-prompt").arg(sys);
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        // Clear CLAUDECODE env var so the child doesn't think it's nested.
        cmd.env_remove("CLAUDECODE");

        cmd
    }
}
