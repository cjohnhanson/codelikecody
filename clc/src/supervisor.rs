//! Supervisor: the `clc up` process.
//!
//! Non-agentic. Starts coordinator(s), monitors their health via the
//! coordination DB, restarts crashed ones, surfaces escalations to the human.
//! Coordinators run on trunk as `clc coordinator-run` processes.

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
    coordinators: Vec<CoordinatorState>,
    poll_interval: Duration,
    shutdown: Arc<AtomicBool>,
}

impl Supervisor {
    pub fn new(
        project_dir: &Path,
        main_branch: &str,
        admin_branch: &str,
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

        let coord = Coordination::open(&self.project_dir)
            .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

        let _ = coord.register_agent("supervisor", None);
        let _ = coord.set_status("supervisor", clc_sdk::coordination::AgentStatus::Running);

        // Generate ephemeral CA for mTLS.
        let ca = crate::tls::EphemeralCA::new()
            .map_err(|e| Error::NonBlocking(format!("CA generation: {e}")))?;
        eprintln!("supervisor: ephemeral CA generated");

        // Start the supervisor API server on a dedicated thread.
        let api_project_dir = self.project_dir.clone();
        let api_port = 19100; // TODO: configurable from SupervisorConfig
        let (api_tx, api_rx) = std::sync::mpsc::channel();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("API tokio runtime");

            rt.block_on(async {
                let db_path = api_project_dir.join(".clc").join("coordination.db");
                let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
                let db = clc_sdk::coordination_db::DbBackend::connect(&db_url)
                    .await
                    .expect("coordination DB for API");
                db.create_tables().await.expect("create tables");
                let api_state = Arc::new(crate::supervisor_api::ApiState {
                    db: Arc::new(db),
                    project_dir: api_project_dir,
                });

                match crate::supervisor_api::start(api_state, api_port).await {
                    Ok(addr) => {
                        let _ = api_tx.send(Ok(addr));
                        // Keep the runtime alive so the server runs.
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                        }
                    }
                    Err(e) => {
                        let _ = api_tx.send(Err(format!("{e}")));
                    }
                }
            });
        });

        let api_addr = api_rx
            .recv()
            .map_err(|e| Error::NonBlocking(format!("API server channel: {e}")))?
            .map_err(|e| Error::NonBlocking(format!("API server: {e}")))?;

        eprintln!("supervisor API listening on {api_addr}");

        let shutdown = Arc::clone(&self.shutdown);
        let _ = ctrlc::set_handler(move || {
            shutdown.store(true, Ordering::SeqCst);
            eprintln!("\nsupervisor: shutting down...");
        });

        eprintln!(
            "supervisor started ({} coordinator(s), poll every {:?})",
            self.coordinators.len(),
            self.poll_interval
        );

        for i in 0..self.coordinators.len() {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }
            self.start_coordinator(i);
        }

        loop {
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }

            let mut all_done = true;
            for i in 0..self.coordinators.len() {
                let c = &self.coordinators[i];

                // Check if process is alive.
                if let Some(pid) = c.pid {
                    if !is_process_alive(pid) {
                        // Process exited. Check DB for status.
                        let db_status = coord.get_status(&c.scope.id).ok();
                        match db_status {
                            Some(clc_sdk::coordination::AgentStatus::Completed) => {
                                eprintln!("supervisor: coordinator '{}' completed", c.scope.id);
                            }
                            _ => {
                                // Crashed or stopped — restart.
                                all_done = false;
                                self.restart_coordinator(i);
                            }
                        }
                    } else {
                        all_done = false;
                    }
                } else {
                    all_done = false;
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

    fn start_coordinator(&mut self, idx: usize) {
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

        match scope.workspace {
            crate::config::WorkspaceType::Docker => {
                cmd.arg("--workspace").arg("docker");
                if let Some(ref image) = scope.docker_image {
                    cmd.arg("--docker-image").arg(image);
                }
            }
            crate::config::WorkspaceType::Worktree => {}
        }

        // Pass API port so coordinator can set up reverse tunnels for Docker workers.
        cmd.env("CLC_API_PORT", "19100");

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

    fn restart_coordinator(&mut self, idx: usize) {
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
        self.start_coordinator(idx);
    }
}

fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid.cast_signed()), None).is_ok()
}
