//! Supervisor: the `clc up` process.
//!
//! Non-agentic. Starts coordinator(s) in workspaces, monitors their health
//! via the coordination DB, restarts crashed ones, surfaces escalations to
//! the human.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::{CoordinatorScope, SupervisorConfig};
use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;

struct CoordinatorState {
    scope: CoordinatorScope,
    pid: Option<u32>,
    resume_count: u32,
    max_resumes: u32,
}

pub struct Supervisor {
    project_dir: PathBuf,
    main_branch: String,
    admin_branch: String,
    #[allow(dead_code)] // Passed to coordinator processes via config.
    worker_perm_defaults: Vec<String>,
    #[allow(dead_code)]
    worker_perm_deny: Vec<String>,
    coordinators: Vec<CoordinatorState>,
    poll_interval: Duration,
    shutdown: Arc<AtomicBool>,
}

impl Supervisor {
    pub fn new(
        project_dir: &Path,
        main_branch: &str,
        admin_branch: &str,
        worker_perm_defaults: &[String],
        worker_perm_deny: &[String],
        config: &SupervisorConfig,
    ) -> Self {
        let coordinators = config
            .coordinators
            .iter()
            .map(|scope| CoordinatorState {
                scope: scope.clone(),
                pid: None,
                resume_count: 0,
                max_resumes: 5,
            })
            .collect();

        Self {
            project_dir: project_dir.to_path_buf(),
            main_branch: main_branch.to_string(),
            admin_branch: admin_branch.to_string(),
            worker_perm_defaults: worker_perm_defaults.to_vec(),
            worker_perm_deny: worker_perm_deny.to_vec(),
            coordinators,
            poll_interval: Duration::from_secs(config.poll_interval),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        let git_state = git::detect(&self.project_dir, &self.main_branch, &self.admin_branch)
            .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

        if !git_state.is_main {
            return Err(Error::NonBlocking(format!(
                "supervisor must run from the main branch (currently on '{}')",
                git_state.branch
            )));
        }

        // Open coordination DB.
        let coord = Coordination::open(&self.project_dir)
            .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

        // Register the supervisor.
        let _ = coord.register_agent("supervisor", None);
        let _ = coord.set_status("supervisor", clc_sdk::coordination::AgentStatus::Running);

        // Install signal handler.
        let shutdown = Arc::clone(&self.shutdown);
        let _ = ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
            eprintln!("\nsupervisor: shutting down...");
        });

        eprintln!(
            "supervisor started ({} coordinator scope(s), poll every {:?})",
            self.coordinators.len(),
            self.poll_interval
        );

        // Start all coordinators.
        for i in 0..self.coordinators.len() {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }
            self.start_coordinator(i, &coord);
        }

        // Main loop.
        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            // Check coordinator health.
            let mut all_done = true;
            for i in 0..self.coordinators.len() {
                let status = coord.get_status(&self.coordinators[i].scope.id);
                match status {
                    Ok(clc_sdk::coordination::AgentStatus::Completed) => {
                        // This scope is done.
                    }
                    Ok(clc_sdk::coordination::AgentStatus::Running) => {
                        all_done = false;
                        // Check if process is actually alive.
                        if let Some(pid) = self.coordinators[i].pid {
                            if !is_process_alive(pid) {
                                eprintln!(
                                    "supervisor: coordinator '{}' crashed (pid {pid})",
                                    self.coordinators[i].scope.id
                                );
                                let _ = coord.set_status(
                                    &self.coordinators[i].scope.id,
                                    clc_sdk::coordination::AgentStatus::Failed,
                                );
                                self.restart_coordinator(i, &coord);
                            }
                        }
                    }
                    Ok(clc_sdk::coordination::AgentStatus::Failed | clc_sdk::coordination::AgentStatus::Stopped) => {
                        all_done = false;
                        self.restart_coordinator(i, &coord);
                    }
                    Ok(clc_sdk::coordination::AgentStatus::Pending) => {
                        all_done = false;
                    }
                    Err(_) => {
                        all_done = false;
                    }
                }
            }

            // Surface escalations.
            if let Ok(escalations) = coord.pending_permissions("admin") {
                for msg in &escalations {
                    if let clc_sdk::coordination::MessageKind::PermissionRequest {
                        ref tool_name,
                        ref reason,
                    } = msg.kind
                    {
                        let worker_id = tool_name
                            .strip_prefix("escalation:")
                            .unwrap_or(&msg.from);
                        eprintln!(
                            "[ESCALATION] worker '{worker_id}': {reason}\n  \
                             Grant: clc permissions grant {worker_id} \"<permission>\"\n  \
                             Deny:  clc permissions deny {worker_id} \"<reason>\""
                        );
                    }
                }
            }

            if all_done {
                eprintln!("supervisor: all coordinators completed");
                let _ = coord.set_status(
                    "supervisor",
                    clc_sdk::coordination::AgentStatus::Completed,
                );
                return Ok(());
            }

            thread::sleep(self.poll_interval);
        }

        // Graceful shutdown.
        eprintln!("supervisor: stopping coordinators...");
        for c in &self.coordinators {
            if let Some(pid) = c.pid {
                if is_process_alive(pid) {
                    let nix_pid = nix::unistd::Pid::from_raw(pid.cast_signed());
                    let _ = nix::sys::signal::kill(nix_pid, nix::sys::signal::Signal::SIGTERM);
                }
            }
        }
        let _ = coord.set_status("supervisor", clc_sdk::coordination::AgentStatus::Stopped);

        Ok(())
    }

    fn start_coordinator(&mut self, idx: usize, _coord: &Coordination) {
        let scope = self.coordinators[idx].scope.clone();
        eprintln!("supervisor: starting coordinator '{}'", scope.id);

        let mut cmd = std::process::Command::new(
            std::env::current_exe().unwrap_or_else(|_| "clc".into()),
        );
        cmd.arg("coordinator-run");
        cmd.arg("--id").arg(&scope.id);
        cmd.arg("--max-workers").arg(scope.max_workers.to_string());
        cmd.arg("--model").arg(&scope.model);

        if let Some(ref project) = scope.project {
            cmd.arg("--project").arg(project);
        }
        if let Some(ref label) = scope.label {
            cmd.arg("--label").arg(label);
        }
        if let Some(ref exclude_label) = scope.exclude_label {
            cmd.arg("--exclude-label").arg(exclude_label);
        }
        for pattern in &scope.auto_grant {
            cmd.arg("--auto-grant").arg(pattern);
        }
        for pattern in &scope.always_escalate {
            cmd.arg("--always-escalate").arg(pattern);
        }

        cmd.current_dir(&self.project_dir);

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                self.coordinators[idx].pid = Some(pid);
                eprintln!("supervisor: coordinator '{}' started (pid {pid})", scope.id);
            }
            Err(e) => {
                eprintln!("supervisor: failed to start coordinator '{}': {e}", scope.id);
            }
        }
    }

    fn restart_coordinator(&mut self, idx: usize, coord: &Coordination) {
        let c = &mut self.coordinators[idx];
        if c.resume_count >= c.max_resumes {
            eprintln!(
                "supervisor: coordinator '{}' exceeded max resumes ({}) — giving up",
                c.scope.id, c.max_resumes
            );
            return;
        }
        c.resume_count += 1;
        eprintln!(
            "supervisor: restarting coordinator '{}' ({}/{})",
            c.scope.id, c.resume_count, c.max_resumes
        );
        // Re-register so the coordinator can re-register without duplicate error.
        // The coordinator handles this itself — just start a new process.
        self.start_coordinator(idx, coord);
    }
}

fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid.cast_signed()),
        None,
    )
    .is_ok()
}
