//! Supervisor: the `clc up` process.
//!
//! Non-agentic. Starts coordinator(s) in workspaces via the Workspace trait,
//! monitors their health via the coordination DB, restarts crashed ones,
//! surfaces escalations to the human.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use clc_sdk::agent::{AgentConfig, ClaudeCodeAgent};
use clc_sdk::workspace::{Workspace, WorkspaceConfig};

use crate::config::{CoordinatorScope, SupervisorConfig};
use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;
use crate::workspace::WorktreeWorkspace;

struct CoordinatorState {
    scope: CoordinatorScope,
    workspace: Option<WorktreeWorkspace>,
    resume_count: u32,
    max_resumes: u32,
}

pub struct Supervisor {
    project_dir: PathBuf,
    main_branch: String,
    admin_branch: String,
    worker_perm_defaults: Vec<String>,
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
                workspace: None,
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

        let coord = Coordination::open(&self.project_dir)
            .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

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

            let mut all_done = true;
            for i in 0..self.coordinators.len() {
                let scope_id = &self.coordinators[i].scope.id;
                let status = coord.get_status(scope_id);
                match status {
                    Ok(clc_sdk::coordination::AgentStatus::Completed) => {}
                    Ok(clc_sdk::coordination::AgentStatus::Running) => {
                        all_done = false;
                    }
                    Ok(
                        clc_sdk::coordination::AgentStatus::Failed
                        | clc_sdk::coordination::AgentStatus::Stopped,
                    ) => {
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
        for c in &mut self.coordinators {
            if let Some(ref mut ws) = c.workspace {
                let _ = ws.stop();
            }
        }
        let _ = coord.set_status("supervisor", clc_sdk::coordination::AgentStatus::Stopped);

        Ok(())
    }

    fn start_coordinator(&mut self, idx: usize, coord: &Coordination) {
        let scope = self.coordinators[idx].scope.clone();
        eprintln!("supervisor: starting coordinator '{}'", scope.id);

        // Seed permissions for the coordinator workspace.
        let _ = crate::permissions::seed_defaults(
            &self.project_dir,
            &self.worker_perm_defaults,
            &self.worker_perm_deny,
        );

        // Build coordinator system prompt with scope context.
        let system_prompt = build_coordinator_system_prompt(&scope);
        let initial_prompt = build_coordinator_initial_prompt(&scope);

        let config = WorkspaceConfig {
            tisket_id: scope.id.clone(),
            project_dir: self.project_dir.clone(),
            main_branch: self.main_branch.clone(),
            agent_config: AgentConfig {
                model: scope.model.clone(),
                system_prompt,
                initial_prompt,
                extra_args: vec![],
            },
        };

        let mut workspace = WorktreeWorkspace::new(
            config,
            Box::new(ClaudeCodeAgent::new()),
        );

        match workspace.start() {
            Ok(()) => {
                eprintln!("supervisor: coordinator '{}' started", scope.id);
                // Register in coordination DB.
                let _ = coord.register_agent(&scope.id, Some("supervisor"));
                let _ = coord.set_status(
                    &scope.id,
                    clc_sdk::coordination::AgentStatus::Running,
                );
                self.coordinators[idx].workspace = Some(workspace);
            }
            Err(e) => {
                eprintln!(
                    "supervisor: failed to start coordinator '{}': {e}",
                    scope.id
                );
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
        self.start_coordinator(idx, coord);
    }
}

fn build_coordinator_system_prompt(scope: &CoordinatorScope) -> String {
    let mut prompt = String::from(
        "You are a coordinator agent managing autonomous workers. \
         Your tools are clc commands: `clc dispatch`, `clc land`, \
         `clc workers`, `clc worker <id> check`, `clc worker <id> send`, \
         `clc permissions grant/deny/list`. \
         Monitor workers, handle permission requests, land completed work, \
         and resume stuck workers.",
    );

    if !scope.auto_grant.is_empty() {
        prompt.push_str("\n\nAuto-grant these permission patterns without asking: ");
        prompt.push_str(&scope.auto_grant.join(", "));
    }

    if !scope.always_escalate.is_empty() {
        prompt.push_str("\n\nAlways escalate these to the admin: ");
        prompt.push_str(&scope.always_escalate.join(", "));
    }

    prompt
}

fn build_coordinator_initial_prompt(scope: &CoordinatorScope) -> String {
    let mut prompt = format!(
        "You are coordinator '{}'. ",
        scope.id
    );

    if let Some(ref project) = scope.project {
        prompt.push_str(&format!("Scope: project '{project}'. "));
    }
    if let Some(ref label) = scope.label {
        prompt.push_str(&format!("Label filter: '{label}'. "));
    }

    prompt.push_str(
        "Check for pickable tiskets with `tisket issue list` and dispatch workers. \
         Monitor their progress. Land completed work. Handle permissions."
    );

    prompt
}
