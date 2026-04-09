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

/// Maximum time a reviewer can be in `Running` status before being considered
/// dead (e.g. the host process crashed or was OOM-killed between registering
/// the agent and recording its result). Host-side reviewers via `claude --print`
/// typically complete in under 2 minutes; 10 minutes is generous.
const REVIEWER_TIMEOUT: Duration = Duration::from_secs(600);

struct CoordinatorState {
    scope: CoordinatorScope,
    pid: Option<u32>,
    resume_count: u32,
    max_resumes: u32,
    /// Keeps the SSH workspace alive for Docker coordinators. The reverse tunnel
    /// runs on the workspace's tokio runtime — dropping this kills the tunnel.
    _workspace: Option<crate::ssh_workspace::SSHWorkspace>,
}

struct WorkerState {
    tisket_id: String,
    #[allow(dead_code)]
    coordinator_id: String,
    #[allow(dead_code)]
    model: String,
    workspace: Option<crate::ssh_workspace::SSHWorkspace>,
}

pub struct Supervisor {
    project_dir: PathBuf,
    main_branch: String,
    admin_branch: String,
    coordinators: Vec<CoordinatorState>,
    workers: Vec<WorkerState>,
    poll_interval: Duration,
    api_port: u16,
    tunnel_base_port: u16,
    shutdown: Arc<AtomicBool>,
    /// Workflow name → full definition (phase graph + reviews).
    workflow_defs: std::collections::HashMap<String, crate::config::WorkflowDef>,
    /// OAuth token for Claude API authentication (passed to reviewers).
    oauth_token: Option<String>,
    /// Message IDs of escalations already printed this supervisor run. Used
    /// to dedupe the `[ESCALATION]` output — without this, every tick
    /// re-prints all pending escalations from the DB, including stale ones
    /// from prior test runs, filling the log with repeated messages.
    shown_escalations: std::collections::HashSet<String>,
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
                _workspace: None,
            })
            .collect();

        Self {
            project_dir: project_dir.to_path_buf(),
            main_branch: main_branch.to_string(),
            admin_branch: admin_branch.to_string(),
            coordinators,
            workers: Vec::new(),
            poll_interval: Duration::from_secs(config.poll_interval),
            api_port: config.api_port,
            tunnel_base_port: config.tunnel_base_port,
            shutdown: Arc::new(AtomicBool::new(false)),
            workflow_defs: config.workflows.clone(),
            oauth_token: std::env::var("CLC_CLAUDE_CODE_OAUTH_TOKEN")
                .or_else(|_| std::env::var("CLAUDE_CODE_OAUTH_TOKEN"))
                .ok()
                .or_else(|| {
                    let token_path = dirs::home_dir()?.join(".claude").join("token");
                    std::fs::read_to_string(token_path).ok().map(|t| t.trim().to_string())
                }),
            shown_escalations: std::collections::HashSet::new(),
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

        // Reset stale agents from prior runs. Everything non-terminal (Pending,
        // Running) is stale — the supervisor just started, so no agents are
        // actually running. Must happen BEFORE the API server starts so the
        // API's DB connection sees the cleaned-up state.
        let all_agents = coord.list_agents(None).unwrap_or_default();
        eprintln!("supervisor: found {} agent(s) in DB", all_agents.len());
        let mut reset_count = 0;
        for (id, status) in &all_agents {
            if matches!(
                status,
                clc_sdk::coordination::AgentStatus::Pending
                    | clc_sdk::coordination::AgentStatus::Running
            ) {
                let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Stopped);
                reset_count += 1;
            }
        }
        if reset_count > 0 {
            eprintln!("supervisor: reset {reset_count} stale agent(s) from prior run");
        }

        let _ = coord.register_agent("supervisor", None);
        let _ = coord.set_status("supervisor", clc_sdk::coordination::AgentStatus::Running);

        // Generate ephemeral CA for mTLS.
        let ca = crate::tls::EphemeralCA::new()
            .map_err(|e| Error::NonBlocking(format!("CA generation: {e}")))?;
        let tls_config = ca
            .server_tls_config()
            .map_err(|e| Error::NonBlocking(format!("TLS config: {e}")))?;

        // Write CA to disk so coordinators can sign worker certs with the same CA.
        let ca_cert_path = self.project_dir.join(".clc").join("ca-cert.pem");
        let ca_key_path = self.project_dir.join(".clc").join("ca-key.pem");
        std::fs::write(&ca_cert_path, &ca.ca_cert_pem)
            .map_err(|e| Error::NonBlocking(format!("write CA cert: {e}")))?;
        std::fs::write(&ca_key_path, &ca.ca_key_pem)
            .map_err(|e| Error::NonBlocking(format!("write CA key: {e}")))?;

        eprintln!("supervisor: ephemeral CA generated, mTLS configured");

        // Start the supervisor API server on a dedicated thread.
        let api_project_dir = self.project_dir.clone();
        let api_workflows = self.workflow_defs.clone();
        let api_port = self.api_port;
        let api_tls = tls_config;
        let (api_tx, api_rx) = std::sync::mpsc::channel();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(8)
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
                    workflows: api_workflows,
                });

                match crate::supervisor_api::start(api_state, api_port, Some(api_tls)).await {
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

                if let Some(pid) = c.pid {
                    // Local coordinator: check process health.
                    if !is_process_alive(pid) {
                        let db_status = coord.get_status(&c.scope.id).ok();
                        match db_status {
                            Some(clc_sdk::coordination::AgentStatus::Completed) => {
                                eprintln!("supervisor: coordinator '{}' completed", c.scope.id);
                            }
                            _ => {
                                all_done = false;
                                self.restart_coordinator(i);
                            }
                        }
                    } else {
                        all_done = false;
                    }
                } else if c._workspace.is_some() {
                    // Docker coordinator: check DB status.
                    let db_status = coord.get_status(&c.scope.id).ok();
                    match db_status {
                        Some(clc_sdk::coordination::AgentStatus::Completed) => {
                            eprintln!("supervisor: coordinator '{}' completed", c.scope.id);
                        }
                        _ => {
                            all_done = false;
                        }
                    }
                } else {
                    // Not started yet.
                    all_done = false;
                }
            }

            // Start pending workers. Coordinators dispatch via POST /dispatch
            // which registers the agent as Pending. The supervisor picks up
            // pending workers here and starts Docker workspaces for them.
            self.start_pending_workers(&coord);

            // Check workers for review gates and spawn reviewer agents.
            self.handle_reviews(&coord);

            // Land completed workers. Fetch the git pack from the worker's
            // container, import into the host repo, and attempt ff-merge.
            self.land_completed_workers(&coord);

            // Surface escalations. Dedupe via `shown_escalations`: each
            // message ID is printed at most once per supervisor run, so
            // stale messages from prior test runs don't flood the log
            // every polling tick.
            if let Ok(escalations) = coord.pending_permissions("admin") {
                for msg in &escalations {
                    if !is_new_escalation(&mut self.shown_escalations, &msg.id) {
                        continue;
                    }
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

        if scope.image.is_some() {
            self.start_coordinator_docker(idx, &scope);
        } else {
            self.start_coordinator_local(idx, &scope);
        }
    }

    /// Start a coordinator as a local process on the host.
    fn start_coordinator_local(&mut self, idx: usize, scope: &crate::config::CoordinatorScope) {
        let mut cmd = std::process::Command::new(
            std::env::current_exe().unwrap_or_else(|_| "clc".into()),
        );
        cmd.arg("coordinator-run");
        self.append_coordinator_args(&mut cmd, scope);

        cmd.env("CLC_API_PORT", self.api_port.to_string());
        cmd.env("CLC_CA_CERT", self.project_dir.join(".clc").join("ca-cert.pem"));
        cmd.env("CLC_CA_KEY", self.project_dir.join(".clc").join("ca-key.pem"));
        cmd.current_dir(&self.project_dir);

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                self.coordinators[idx].pid = Some(pid);
                eprintln!("supervisor: coordinator '{}' started locally (pid {pid})", scope.id);
            }
            Err(e) => {
                eprintln!("supervisor: failed to start coordinator '{}': {e}", scope.id);
            }
        }
    }

    /// Start a coordinator inside a Docker container via SSHWorkspace.
    fn start_coordinator_docker(&mut self, idx: usize, scope: &crate::config::CoordinatorScope) {
        use crate::ssh_workspace::{DockerEnvironment, SSHWorkspace, SSHWorkspaceConfig};
        use clc_sdk::workspace::{WorkspaceConfig, Workspace};
        use clc_sdk::agent::AgentConfig;

        let Some(image) = scope.image.as_deref() else {
            eprintln!("supervisor: coordinator '{}' has no image configured", scope.id);
            return;
        };
        let tunnel_port = self.tunnel_base_port + idx as u16;

        // Build the coordinator-run command that will execute inside the container.
        let mut start_cmd = vec![
            "clc".to_string(),
            "coordinator-run".to_string(),
            "--id".to_string(),
            scope.id.clone(),
            "--max-workers".to_string(),
            scope.max_workers.to_string(),
            "--model".to_string(),
            scope.model.clone(),
        ];
        if let Some(ref project) = scope.project {
            start_cmd.push("--project".to_string());
            start_cmd.push(project.clone());
        }
        if let Some(ref label) = scope.label {
            start_cmd.push("--label".to_string());
            start_cmd.push(label.clone());
        }
        if let Some(ref exclude_label) = scope.exclude_label {
            start_cmd.push("--exclude-label".to_string());
            start_cmd.push(exclude_label.clone());
        }
        for pattern in &scope.auto_grant {
            start_cmd.push("--auto-grant".to_string());
            start_cmd.push(pattern.clone());
        }
        for pattern in &scope.always_escalate {
            start_cmd.push("--always-escalate".to_string());
            start_cmd.push(pattern.clone());
        }
        if let Some(ref workflow) = scope.workflow {
            start_cmd.push("--workflow".to_string());
            start_cmd.push(workflow.clone());
        }
        // Coordinator in Docker dispatches workers via API, not local process.
        start_cmd.push("--workspace".to_string());
        start_cmd.push("docker".to_string());
        if let Some(ref img) = scope.image {
            start_cmd.push("--docker-image".to_string());
            start_cmd.push(img.clone());
        }

        let ca_cert_path = self.project_dir.join(".clc").join("ca-cert.pem");
        let ca_key_path = self.project_dir.join(".clc").join("ca-key.pem");
        let ca = match (
            std::fs::read_to_string(&ca_cert_path),
            std::fs::read_to_string(&ca_key_path),
        ) {
            (Ok(cert_pem), Ok(key_pem)) => {
                match crate::tls::EphemeralCA::from_pem(&cert_pem, &key_pem) {
                    Ok(ca) => std::sync::Arc::new(ca),
                    Err(e) => {
                        eprintln!("supervisor: failed to load CA for coordinator '{}': {e}", scope.id);
                        return;
                    }
                }
            }
            _ => {
                eprintln!("supervisor: CA files not found for coordinator '{}'", scope.id);
                return;
            }
        };

        let env = match DockerEnvironment::new(image, &self.project_dir, &scope.id) {
            Ok(env) => env,
            Err(e) => {
                eprintln!("supervisor: docker env failed for coordinator '{}': {e}", scope.id);
                return;
            }
        };

        let ws_config = WorkspaceConfig {
            agent_config: AgentConfig {
                model: scope.model.clone(),
                system_prompt: String::new(),
                initial_prompt: String::new(),
                extra_args: vec![],
                allowed_tools: vec![],
            },
            tisket_id: scope.id.clone(),
            project_dir: self.project_dir.clone(),
            main_branch: self.main_branch.clone(),
        };

        let ssh_config = SSHWorkspaceConfig {
            workspace_config: ws_config,
            ca,
            api_port: self.api_port,
            oauth_token: None,
            start_command: Some(start_cmd),
        };

        let mut workspace = match SSHWorkspace::new(
            ssh_config,
            Box::new(env),
            tunnel_port,
        ) {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("supervisor: SSH workspace failed for coordinator '{}': {e}", scope.id);
                return;
            }
        };

        match workspace.start() {
            Ok(()) => {
                eprintln!("supervisor: coordinator '{}' started in Docker", scope.id);
                self.coordinators[idx]._workspace = Some(workspace);
            }
            Err(e) => {
                eprintln!("supervisor: docker start failed for coordinator '{}': {e}", scope.id);
            }
        }
    }

    fn append_coordinator_args(
        &self,
        cmd: &mut std::process::Command,
        scope: &crate::config::CoordinatorScope,
    ) {
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
        if let Some(ref workflow) = scope.workflow {
            cmd.arg("--workflow").arg(workflow);
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

    /// Poll DB for Pending agents that aren't coordinators and start workspaces.
    fn start_pending_workers(&mut self, coord: &Coordination) {
        let all_agents = coord.list_agents(None).unwrap_or_default();
        let coordinator_ids: Vec<String> = self.coordinators.iter().map(|c| c.scope.id.clone()).collect();
        let already_launched: Vec<String> = self.workers.iter().map(|w| w.tisket_id.clone()).collect();

        // Collect work items first to avoid borrow conflicts with self.
        let mut to_launch: Vec<(String, String, String, String)> = Vec::new();

        for (id, status) in &all_agents {
            if *status != clc_sdk::coordination::AgentStatus::Pending {
                continue;
            }
            if id == "supervisor" || coordinator_ids.iter().any(|cid| cid == id) {
                continue;
            }
            if already_launched.iter().any(|wid| wid == id) {
                continue;
            }

            // Find which coordinator owns this worker to get config.
            let parent_scope = self.coordinators.iter().find(|c| {
                coord
                    .list_agents(Some(&c.scope.id))
                    .unwrap_or_default()
                    .iter()
                    .any(|(aid, _)| aid == id)
            });

            match parent_scope {
                Some(c) => {
                    let Some(image) = c.scope.image.as_deref() else {
                        eprintln!("supervisor: coordinator '{}' has no image for worker '{id}'", c.scope.id);
                        continue;
                    };
                    let image = image.to_string();
                    to_launch.push((id.clone(), c.scope.model.clone(), image, c.scope.id.clone()));
                }
                None => {
                    eprintln!("supervisor: pending worker '{id}' has no parent coordinator");
                }
            }
        }

        for (tisket_id, model, image, coordinator_id) in to_launch {
            eprintln!("supervisor: starting worker '{tisket_id}' (image: {image})");
            self.start_worker_docker(&tisket_id, &model, &image, &coordinator_id, coord);
        }
    }

    /// Start a worker inside a Docker container.
    fn start_worker_docker(
        &mut self,
        tisket_id: &str,
        model: &str,
        image: &str,
        coordinator_id: &str,
        coord: &Coordination,
    ) {
        use crate::ssh_workspace::{DockerEnvironment, SSHWorkspace, SSHWorkspaceConfig};
        use clc_sdk::agent::AgentConfig;
        use clc_sdk::workspace::{Workspace, WorkspaceConfig};

        let tunnel_port = self.tunnel_base_port + self.coordinators.len() as u16 + self.workers.len() as u16;

        let ca_cert_path = self.project_dir.join(".clc").join("ca-cert.pem");
        let ca_key_path = self.project_dir.join(".clc").join("ca-key.pem");
        let ca = match (
            std::fs::read_to_string(&ca_cert_path),
            std::fs::read_to_string(&ca_key_path),
        ) {
            (Ok(cert_pem), Ok(key_pem)) => {
                match crate::tls::EphemeralCA::from_pem(&cert_pem, &key_pem) {
                    Ok(ca) => std::sync::Arc::new(ca),
                    Err(e) => {
                        eprintln!("supervisor: CA load failed for worker '{tisket_id}': {e}");
                        let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
                        return;
                    }
                }
            }
            _ => {
                eprintln!("supervisor: CA files not found for worker '{tisket_id}'");
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
                return;
            }
        };

        let env = match DockerEnvironment::new(image, &self.project_dir, tisket_id) {
            Ok(env) => env,
            Err(e) => {
                eprintln!("supervisor: docker env failed for worker '{tisket_id}': {e}");
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
                return;
            }
        };

        // Read the oauth token: env var first, then ~/.claude/token.
        let oauth_token = std::env::var("CLC_CLAUDE_CODE_OAUTH_TOKEN")
            .or_else(|_| std::env::var("CLAUDE_CODE_OAUTH_TOKEN"))
            .ok()
            .or_else(|| {
                let token_path = dirs::home_dir()?.join(".claude").join("token");
                std::fs::read_to_string(token_path).ok().map(|t| t.trim().to_string())
            });

        let ws_config = WorkspaceConfig {
            agent_config: AgentConfig {
                model: model.to_string(),
                system_prompt: String::new(),
                initial_prompt: String::new(),
                extra_args: vec![],
                allowed_tools: vec![],
            },
            tisket_id: tisket_id.to_string(),
            project_dir: self.project_dir.clone(),
            main_branch: self.main_branch.clone(),
        };

        let ssh_config = SSHWorkspaceConfig {
            workspace_config: ws_config,
            ca,
            api_port: self.api_port,
            oauth_token,
            start_command: None, // Workers use default clc workspace start
        };

        let mut workspace = match SSHWorkspace::new(ssh_config, Box::new(env), tunnel_port) {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("supervisor: SSH workspace failed for worker '{tisket_id}': {e}");
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
                return;
            }
        };

        match workspace.start() {
            Ok(()) => {
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Running);
                eprintln!("supervisor: worker '{tisket_id}' started in Docker");
                self.workers.push(WorkerState {
                    tisket_id: tisket_id.to_string(),
                    coordinator_id: coordinator_id.to_string(),
                    model: model.to_string(),
                    workspace: Some(workspace),
                });
            }
            Err(e) => {
                eprintln!("supervisor: docker start failed for worker '{tisket_id}': {e}");
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
            }
        }
    }

    /// Check workers for pending review gates and spawn reviewer agents.
    ///
    /// Uses the workflow's `required_reviews_from()` to determine which phases
    /// have review gates — no hardcoded phase names. When all required reviews
    /// are approved, advances the phase via the API.
    fn handle_reviews(&mut self, coord: &Coordination) {
        // Fetch all agents once to avoid N+1 queries.
        let all_agents = coord.list_agents(None).unwrap_or_default();

        struct ReviewAction {
            worker_idx: usize,
            worker_id: String,
            agent_names: Vec<String>,
        }
        struct AdvanceAction {
            worker_id: String,
            current_phase: String,
            next_phase: String,
            workflow_name: Option<String>,
        }
        let mut spawn_actions = Vec::new();
        let mut advance_actions = Vec::new();

        for (idx, worker) in self.workers.iter().enumerate() {
            // Only check running workers.
            let status = all_agents
                .iter()
                .find(|(id, _)| *id == worker.tisket_id)
                .map(|(_, s)| s.clone());
            if !matches!(status, Some(clc_sdk::coordination::AgentStatus::Running)) {
                continue;
            }

            // Only process workers whose branch exists locally. A missing
            // branch means either (a) the worker hasn't committed anything
            // yet, or (b) stale DB state from a prior run on another machine.
            // Either way, there's nothing to review and reviewers would just
            // fail with "branch does not exist locally". Skip silently —
            // handle_reviews runs every tick, so we'll retry once the worker
            // actually pushes work.
            if !crate::gix_ops::branch_exists(&self.project_dir, &worker.tisket_id) {
                continue;
            }

            // Get current phase and workflow.
            let phase_info = coord.get_phase(&worker.tisket_id).ok().flatten();
            let Some((ref phase, _, ref workflow_name)) = phase_info else {
                continue;
            };

            // Build the workflow from the definition, or fall back to default.
            let wf = workflow_name
                .as_deref()
                .and_then(|name| self.workflow_defs.get(name))
                .and_then(|def| crate::workflow::Workflow::new(def).ok())
                .unwrap_or_else(crate::workflow::Workflow::default_tdd);

            // Check if the current phase has review-gated transitions.
            if !wf.has_review_gate(phase) {
                continue;
            }

            // Get reviewer agent names from the transition's review field.
            let agent_names = wf.reviewers_from(phase);

            if agent_names.is_empty() {
                continue;
            }

            // Skip if reviewers already running for this worker, but mark
            // timed-out reviewers as failed so they don't block forever.
            let reviewer_prefix = format!("{}-reviewer-", worker.tisket_id);
            let mut has_active_reviewers = false;
            for (id, s) in &all_agents {
                if !id.starts_with(&reviewer_prefix) {
                    continue;
                }
                if !matches!(
                    s,
                    clc_sdk::coordination::AgentStatus::Running
                        | clc_sdk::coordination::AgentStatus::Pending
                ) {
                    continue;
                }
                // Check if this reviewer has exceeded the timeout.
                let timed_out = coord
                    .get_agent_created_at(id)
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age > REVIEWER_TIMEOUT);
                if timed_out {
                    eprintln!(
                        "supervisor: reviewer '{id}' timed out (>{} min), marking failed",
                        REVIEWER_TIMEOUT.as_secs() / 60
                    );
                    let _ = coord.set_status(
                        id,
                        clc_sdk::coordination::AgentStatus::Failed,
                    );
                } else {
                    has_active_reviewers = true;
                }
            }
            if has_active_reviewers {
                continue;
            }

            // Check if all reviews are already approved.
            let all_approved = agent_names.iter().all(|name| {
                crate::review::check_review_requirements(
                    &self.project_dir,
                    &worker.tisket_id,
                    &[name.clone()],
                )
                .is_ok()
            });

            if all_approved {
                // Find the transition whose review gate is now satisfied.
                if let Some(phase_def) = wf.phase_def(phase) {
                    if let Some(transitions) = &phase_def.transitions {
                        // Pick the review-gated transition.
                        // If none has reviewers, fall back to the first.
                        let target = transitions
                            .iter()
                            .find(|t| !t.reviewers().is_empty())
                            .or_else(|| transitions.first())
                            .map(|t| t.target().to_string());
                        if let Some(target) = target {
                            advance_actions.push(AdvanceAction {
                                worker_id: worker.tisket_id.clone(),
                                current_phase: phase.clone(),
                                next_phase: target,
                                workflow_name: workflow_name.clone(),
                            });
                        }
                    }
                }
                continue;
            }

            spawn_actions.push(ReviewAction {
                worker_idx: idx,
                worker_id: worker.tisket_id.clone(),
                agent_names,
            });
        }

        // Advance workers with all reviews approved, then resume them.
        for action in &advance_actions {
            eprintln!(
                "supervisor: all reviews approved for '{}', advancing {} → {}",
                action.worker_id, action.current_phase, action.next_phase
            );
            let _ = coord.set_phase_via_db(
                &action.worker_id,
                &action.next_phase,
                action.workflow_name.as_deref(),
            );
        }

        // Resume workers that were advanced (they stopped at the review gate).
        for action in &advance_actions {
            if let Some(worker) = self.workers.iter_mut().find(|w| w.tisket_id == action.worker_id) {
                if let Some(ref mut ws) = worker.workspace {
                    let project = crate::ssh_workspace::REMOTE_PROJECT_DIR;
                    let resume_msg = format!(
                        "Review approved. Phase advanced to '{}'. Continue working.",
                        action.next_phase
                    );
                    eprintln!("supervisor: resuming '{}' after review", action.worker_id);
                    // Write JSON-formatted InputMessage to the worker's stdin pipe.
                    let input = claude_code::protocol::InputMessage::user(&resume_msg);
                    if let Ok(json) = serde_json::to_string(&input) {
                        let escaped = json.replace('\'', "'\\''");
                        let _ = ws.exec(&format!(
                            "printf '%s\\n' '{escaped}' > {project}/.clc/worker/stdin.pipe 2>/dev/null",
                        ));
                    }
                }
            }
        }

        // Fetch all messages once for diff-hash dedup checks. Reading
        // from cursor 0 gives the full history — ReviewResult messages
        // include diff_hash when the supervisor wrote the verdict.
        let all_messages: Vec<clc_sdk::coordination::Message> = spawn_actions
            .iter()
            .flat_map(|a| {
                coord
                    .recv(&a.worker_id, &clc_sdk::coordination::Cursor::default())
                    .map(|(msgs, _)| msgs)
                    .unwrap_or_default()
            })
            .collect();

        // Spawn reviewers for workers that need them.
        for action in spawn_actions {
            let Some(ref mut ws) = self.workers[action.worker_idx].workspace else {
                continue;
            };

            // Get the diff from the worker's container (once per worker,
            // shared across all reviewer agents).
            let project = crate::ssh_workspace::REMOTE_PROJECT_DIR;
            let diff = ws.exec(
                &format!("cd {project} && git diff {}..HEAD --stat && echo '---' && git diff {}..HEAD", self.main_branch, self.main_branch),
            ).unwrap_or_default();

            // Diff-hash dedup: if the diff hasn't changed since the last
            // review, skip re-review — the verdict would be the same.
            let current_hash = hash_diff(&diff);
            if should_skip_review(&all_messages, &action.worker_id, &current_hash) {
                eprintln!(
                    "supervisor: skipping re-review for '{}' — diff unchanged since last review",
                    action.worker_id
                );
                continue;
            }

            let diff_str = String::from_utf8_lossy(&diff);
            let diff_truncated = if diff_str.len() > 50000 {
                format!("{}...(truncated)", &diff_str[..50000])
            } else {
                diff_str.to_string()
            };

            eprintln!(
                "supervisor: spawning {} reviewer(s) for '{}'",
                action.agent_names.len(),
                action.worker_id
            );

            for agent_name in &action.agent_names {
                let reviewer = match crate::reviewer::resolve(&self.project_dir, agent_name) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("supervisor: reviewer '{agent_name}' not found: {e}");
                        continue;
                    }
                };

                let review_prompt = format!(
                    "You are reviewer '{}' performing a review of worker '{}'.\n\n\
                     {}\n\n\
                     Examine the code changes on this branch. When done:\n\
                     - `clc review approve \"comments\"` to approve\n\
                     - `clc review request-changes \"what needs to change\"` to request changes\n\n\
                     You must render exactly one verdict before stopping.",
                    agent_name, action.worker_id, reviewer.prompt
                );

                let reviewer_id = format!("{}-reviewer-{agent_name}", action.worker_id);
                let _ = coord.register_agent(&reviewer_id, Some(&action.worker_id));
                let _ = coord.set_status(&reviewer_id, clc_sdk::coordination::AgentStatus::Running);

                // Build the review prompt with the diff included.
                let full_prompt = format!(
                    "{review_prompt}\n\n## Diff\n\n```\n{diff_truncated}\n```\n\n\
                     Based on the diff above, render your verdict. \
                     Reply with APPROVED or CHANGES_REQUESTED followed by your comments.",
                );

                // Run the reviewer on the HOST in a background thread.
                // The supervisor tick loop continues while the review runs.
                let model = reviewer.spec.model.as_deref()
                    .unwrap_or(crate::config::DEFAULT_REVIEWER_MODEL)
                    .to_string();
                let escaped = full_prompt.replace('\'', "'\\''");
                let oauth_env = self.oauth_token.as_deref()
                    .map(|t| format!("CLAUDE_CODE_OAUTH_TOKEN={t} "))
                    .unwrap_or_default();
                let project_dir = self.project_dir.clone();
                let worker_id = action.worker_id.clone();
                let agent_name_owned = agent_name.clone();
                let reviewer_id_owned = reviewer_id.clone();
                let hash = current_hash.clone();

                std::thread::spawn(move || {
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(format!("{oauth_env}claude --model {model} --print '{escaped}'"))
                        .output();

                    // Open a fresh coordination handle for this thread.
                    let Ok(coord) = Coordination::open(&project_dir) else {
                        eprintln!("supervisor: reviewer thread failed to open coordination DB");
                        return;
                    };

                    match output {
                        Ok(out) => {
                            let response = String::from_utf8_lossy(&out.stdout);
                            eprintln!(
                                "supervisor: reviewer '{agent_name_owned}' for '{worker_id}': {}",
                                response.chars().take(100).collect::<String>()
                            );

                            let verdict = if response.to_uppercase().contains("APPROVED")
                                && !response.to_uppercase().contains("CHANGES_REQUESTED")
                            {
                                clc_sdk::coordination::ReviewVerdict::Approved
                            } else {
                                clc_sdk::coordination::ReviewVerdict::ChangesRequested
                            };

                            let _ = coord.send(clc_sdk::coordination::Message {
                                id: format!("review-{agent_name_owned}-{}", std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()),
                                from: reviewer_id_owned.clone(),
                                to: worker_id.clone(),
                                kind: clc_sdk::coordination::MessageKind::ReviewResult {
                                    request_id: String::new(),
                                    review_type: agent_name_owned.clone(),
                                    verdict,
                                    comments: response.chars().take(2000).collect(),
                                    diff_hash: Some(hash),
                                },
                            timestamp: std::time::SystemTime::now(),
                        });

                        let _ = coord.set_status(
                            &reviewer_id_owned,
                            clc_sdk::coordination::AgentStatus::Completed,
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "supervisor: reviewer '{agent_name_owned}' failed for '{worker_id}': {e}",
                        );
                        let _ = coord.set_status(
                            &reviewer_id_owned,
                            clc_sdk::coordination::AgentStatus::Failed,
                        );
                    }
                }
                }); // end thread::spawn
            }
        }
    }

    /// Land completed workers: fetch pack from container, import, merge to trunk.
    fn land_completed_workers(&mut self, coord: &Coordination) {
        let all_agents = coord.list_agents(None).unwrap_or_default();
        let coordinator_ids: Vec<String> =
            self.coordinators.iter().map(|c| c.scope.id.clone()).collect();

        for (id, status) in &all_agents {
            if *status != clc_sdk::coordination::AgentStatus::Completed {
                continue;
            }
            if id == "supervisor" || coordinator_ids.iter().any(|cid| cid == id) {
                continue;
            }

            // Find the worker state with the SSH workspace.
            let worker = self.workers.iter_mut().find(|w| w.tisket_id == *id);
            let Some(worker) = worker else {
                continue;
            };
            let Some(ref mut ws) = worker.workspace else {
                continue;
            };

            eprintln!("supervisor: landing worker '{id}'");

            // Verify the worker has meaningful commits (not just pickup + finalize).
            let project = crate::ssh_workspace::REMOTE_PROJECT_DIR;
            let commit_check = ws.exec(
                &format!("cd {project} && git log --oneline {}..HEAD --no-merges | grep -cv 'clc: pickup\\|clc: finalize'", self.main_branch),
            );
            let meaningful_commits = commit_check
                .ok()
                .and_then(|out| String::from_utf8(out).ok())
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);

            if meaningful_commits == 0 {
                eprintln!("supervisor: rejecting landing for '{id}' — no meaningful commits");
                let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Failed);
                continue;
            }
            eprintln!("supervisor: '{id}' has {meaningful_commits} meaningful commit(s)");

            // 1. Tar .git/ on the container and get the ref tip.
            //    Using raw tar + git rev-parse instead of `clc workspace export`
            //    because gix::open hangs in certain container environments.
            match ws.exec(
                &format!("cd {project} && tar czf /tmp/repo-export.tar.gz .git/"),
            ) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("supervisor: tar .git from '{id}' failed: {e}");
                    continue;
                }
            };

            let ref_output = match ws.exec(
                &format!("cd {project} && git rev-parse refs/heads/{id}"),
            ) {
                Ok(data) => String::from_utf8_lossy(&data).trim().to_string(),
                Err(e) => {
                    eprintln!("supervisor: rev-parse '{id}' failed: {e}");
                    continue;
                }
            };

            // 2. Read the tar file via SSH. This reads chunks through the
            //    channel — cat handles large files because it streams.
            let pack_data = match ws.exec("cat /tmp/repo-export.tar.gz") {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("supervisor: read tar from '{id}' failed: {e}");
                    continue;
                }
            };

            let refs = vec![(ref_output, format!("refs/heads/{id}"))];
            eprintln!("supervisor: got {} bytes from '{id}'", pack_data.len());

            // 2. Import the pack into the host repo.
            if let Err(e) = crate::git_pack::import_pack(&pack_data, &refs, &self.project_dir) {
                eprintln!("supervisor: import pack for '{id}' failed: {e}");
                continue;
            }
            eprintln!("supervisor: imported pack from '{id}'");

            // 3. ff_merge handles rebase-then-merge internally via gix_ops::rebase_onto_head
            //    when the branch isn't a direct fast-forward of trunk.
            match crate::gix_ops::ff_merge(&self.project_dir, id) {
                Ok(()) => {
                    eprintln!("supervisor: landed '{id}' on trunk");
                    let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Stopped);
                    // Clean up the worker container.
                    if let Some(ref mut ws) = worker.workspace {
                        let _ = clc_sdk::workspace::Workspace::stop(ws);
                    }
                    worker.workspace = None;
                }
                Err(e) => {
                    // Conflict — leave as Completed so the coordinator can
                    // tell the worker to merge main and retry.
                    eprintln!("supervisor: merge conflict landing '{id}': {e}");
                    // Send a message to the coordinator about the conflict.
                    let _ = coord.send(clc_sdk::coordination::Message {
                        id: String::new(),
                        from: "supervisor".to_string(),
                        to: id.to_string(),
                        kind: clc_sdk::coordination::MessageKind::Text(format!(
                            "Landing failed: {e}\nMerge the latest main into your branch and retry `clc done`."
                        )),
                        timestamp: std::time::SystemTime::now(),
                    });
                }
            }
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid.cast_signed()), None).is_ok()
}

/// Compute a SHA-1 hex digest of the given diff bytes. Used to detect
/// when a worker's diff hasn't changed between supervisor ticks so
/// re-reviewing can be skipped.
fn hash_diff(diff: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(diff);
    format!("{:x}", hasher.finalize())
}

/// Check whether re-review should be skipped for a worker. Returns true
/// when the most recent `ReviewResult` addressed to this worker has a
/// `diff_hash` that matches `current_diff_hash` — meaning the diff
/// hasn't changed since the last review, so the verdict would be the
/// same.
///
/// Only non-approved verdicts cause skipping. If the last verdict was
/// `Approved`, the review gate logic will advance the worker and no
/// re-review is spawned anyway.
fn should_skip_review(
    messages: &[clc_sdk::coordination::Message],
    worker_id: &str,
    current_diff_hash: &str,
) -> bool {
    // Walk messages in reverse to find the most recent ReviewResult
    // addressed to this worker.
    messages
        .iter()
        .rev()
        .filter(|m| m.to == worker_id)
        .find_map(|m| match &m.kind {
            clc_sdk::coordination::MessageKind::ReviewResult { diff_hash, .. } => {
                diff_hash.as_deref()
            }
            _ => None,
        })
        .is_some_and(|prev_hash| prev_hash == current_diff_hash)
}

/// Returns true if this escalation message ID hasn't been printed yet in
/// the current supervisor run. Inserts the ID into `shown` as a side effect,
/// so repeated calls with the same ID return false.
///
/// Used by the main supervisor loop to dedupe the `[ESCALATION]` output.
/// Without this, every tick would re-print every pending escalation in the
/// coordination DB, including stale messages from prior runs or test
/// harnesses that leaked agent IDs into the DB.
fn is_new_escalation(shown: &mut std::collections::HashSet<String>, msg_id: &str) -> bool {
    shown.insert(msg_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clc_sdk::coordination::{Message, MessageKind, ReviewVerdict};
    use std::time::SystemTime;

    fn review_msg(
        id: &str,
        to: &str,
        verdict: ReviewVerdict,
        diff_hash: Option<&str>,
    ) -> Message {
        Message {
            id: id.into(),
            from: format!("{to}-reviewer-code"),
            to: to.into(),
            kind: MessageKind::ReviewResult {
                request_id: String::new(),
                review_type: "code".into(),
                verdict,
                comments: "test".into(),
                diff_hash: diff_hash.map(String::from),
            },
            timestamp: SystemTime::now(),
        }
    }

    /// Beck's desiderata: isolated, deterministic, fast, specific.
    /// hash_diff produces a stable hex string for a given input.
    #[test]
    fn hash_diff_deterministic() {
        let diff = b"diff --git a/foo.rs b/foo.rs\n+fn bar() {}\n";
        let h1 = hash_diff(diff);
        let h2 = hash_diff(diff);
        assert_eq!(h1, h2, "same input should produce same hash");
        assert_eq!(h1.len(), 40, "SHA-1 hex digest is 40 chars");
    }

    /// Beck's desiderata: behavioral, specific.
    /// Different diffs produce different hashes.
    #[test]
    fn hash_diff_different_inputs() {
        let h1 = hash_diff(b"diff A");
        let h2 = hash_diff(b"diff B");
        assert_ne!(h1, h2, "different inputs should produce different hashes");
    }

    /// Beck's desiderata: behavioral, specific.
    /// Empty diff still produces a valid hash.
    #[test]
    fn hash_diff_empty() {
        let h = hash_diff(b"");
        assert_eq!(h.len(), 40);
    }

    /// Beck's desiderata: behavioral, specific.
    /// When no prior review messages exist, re-review should not be skipped.
    #[test]
    fn should_skip_review_no_messages() {
        let msgs: Vec<Message> = vec![];
        assert!(
            !should_skip_review(&msgs, "worker-1", "abc123"),
            "no messages means nothing to dedup against"
        );
    }

    /// Beck's desiderata: behavioral, specific.
    /// When the most recent ReviewResult has a matching diff_hash,
    /// re-review should be skipped.
    #[test]
    fn should_skip_review_matching_hash() {
        let hash = hash_diff(b"diff content");
        let msgs = vec![review_msg("r1", "worker-1", ReviewVerdict::ChangesRequested, Some(&hash))];
        assert!(
            should_skip_review(&msgs, "worker-1", &hash),
            "matching hash should skip re-review"
        );
    }

    /// Beck's desiderata: behavioral, specific.
    /// When the diff has changed (different hash), re-review should proceed.
    #[test]
    fn should_skip_review_different_hash() {
        let old_hash = hash_diff(b"old diff");
        let new_hash = hash_diff(b"new diff");
        let msgs = vec![review_msg("r1", "worker-1", ReviewVerdict::ChangesRequested, Some(&old_hash))];
        assert!(
            !should_skip_review(&msgs, "worker-1", &new_hash),
            "different hash means diff changed, should re-review"
        );
    }

    /// Beck's desiderata: behavioral, specific.
    /// ReviewResult messages without diff_hash (from worker-side clc review)
    /// should not trigger skip — those are legacy/manual verdicts.
    #[test]
    fn should_skip_review_no_hash_in_message() {
        let msgs = vec![review_msg("r1", "worker-1", ReviewVerdict::ChangesRequested, None)];
        assert!(
            !should_skip_review(&msgs, "worker-1", "abc123"),
            "no diff_hash in message means can't dedup"
        );
    }

    /// Beck's desiderata: isolated, behavioral.
    /// Messages addressed to a different worker should be ignored.
    #[test]
    fn should_skip_review_different_worker() {
        let hash = hash_diff(b"diff");
        let msgs = vec![review_msg("r1", "worker-2", ReviewVerdict::ChangesRequested, Some(&hash))];
        assert!(
            !should_skip_review(&msgs, "worker-1", &hash),
            "messages for other workers should be ignored"
        );
    }

    /// Beck's desiderata: behavioral, specific.
    /// When there are multiple review results, only the most recent matters.
    /// If the latest has a different hash, re-review should proceed even
    /// if an earlier one matches.
    #[test]
    fn should_skip_review_uses_most_recent() {
        let hash_v1 = hash_diff(b"diff v1");
        let hash_v2 = hash_diff(b"diff v2");
        let msgs = vec![
            review_msg("r1", "worker-1", ReviewVerdict::ChangesRequested, Some(&hash_v1)),
            review_msg("r2", "worker-1", ReviewVerdict::ChangesRequested, Some(&hash_v2)),
        ];
        // Current diff matches v1 (old), but latest review was against v2.
        assert!(
            !should_skip_review(&msgs, "worker-1", &hash_v1),
            "should use most recent review, not older ones"
        );
        // Current diff matches v2 (latest review).
        assert!(
            should_skip_review(&msgs, "worker-1", &hash_v2),
            "matching most recent review should skip"
        );
    }

    /// Beck's desiderata: behavioral, specific.
    /// Approved verdicts with matching diff_hash should still cause skip.
    /// (The supervisor's own logic won't reach should_skip_review for
    /// approved workers because check_review_requirements passes, but
    /// the function itself should be purely hash-based — it doesn't
    /// interpret the verdict.)
    #[test]
    fn should_skip_review_approved_with_matching_hash() {
        let hash = hash_diff(b"approved diff");
        let msgs = vec![review_msg("r1", "worker-1", ReviewVerdict::Approved, Some(&hash))];
        assert!(
            should_skip_review(&msgs, "worker-1", &hash),
            "hash match is hash match regardless of verdict"
        );
    }

    /// Beck's desiderata: behavioral.
    /// Non-ReviewResult messages are ignored by should_skip_review.
    #[test]
    fn should_skip_review_ignores_non_review_messages() {
        let hash = hash_diff(b"diff");
        let msgs = vec![Message {
            id: "m1".into(),
            from: "supervisor".into(),
            to: "worker-1".into(),
            kind: MessageKind::Text("hello".into()),
            timestamp: SystemTime::now(),
        }];
        assert!(
            !should_skip_review(&msgs, "worker-1", &hash),
            "non-review messages should be ignored"
        );
    }

    // --- is_new_escalation tests ---

    #[test]
    fn is_new_escalation_first_call_returns_true() {
        let mut shown = std::collections::HashSet::new();
        assert!(is_new_escalation(&mut shown, "msg-1"));
    }

    #[test]
    fn is_new_escalation_repeat_returns_false() {
        let mut shown = std::collections::HashSet::new();
        is_new_escalation(&mut shown, "msg-1");
        assert!(!is_new_escalation(&mut shown, "msg-1"));
    }

    #[test]
    fn is_new_escalation_different_ids_all_return_true() {
        let mut shown = std::collections::HashSet::new();
        assert!(is_new_escalation(&mut shown, "msg-1"));
        assert!(is_new_escalation(&mut shown, "msg-2"));
        assert!(is_new_escalation(&mut shown, "msg-3"));
        assert_eq!(shown.len(), 3);
    }

    #[test]
    fn is_new_escalation_dedups_across_many_repeats() {
        let mut shown = std::collections::HashSet::new();
        // Simulate 1000 polling ticks all seeing the same stale message.
        let mut printed = 0;
        for _ in 0..1000 {
            if is_new_escalation(&mut shown, "stale-msg") {
                printed += 1;
            }
        }
        assert_eq!(printed, 1, "should print stale message exactly once");
    }
}
