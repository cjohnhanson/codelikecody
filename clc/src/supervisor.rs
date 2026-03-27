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
    _workspace: Option<crate::ssh_workspace::SSHWorkspace>,
}

pub struct Supervisor {
    project_dir: PathBuf,
    main_branch: String,
    admin_branch: String,
    coordinators: Vec<CoordinatorState>,
    workers: Vec<WorkerState>,
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
        let api_port = 19100; // TODO: configurable from SupervisorConfig
        let api_tls = tls_config;
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

            // Start pending workers. Coordinators dispatch via POST /dispatch
            // which registers the agent as Pending. The supervisor picks up
            // pending workers here and starts Docker workspaces for them.
            self.start_pending_workers(&coord);

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

        match scope.workspace {
            crate::config::WorkspaceType::Docker => {
                self.start_coordinator_docker(idx, &scope);
            }
            crate::config::WorkspaceType::Worktree => {
                self.start_coordinator_local(idx, &scope);
            }
        }
    }

    /// Start a coordinator as a local process on the host.
    fn start_coordinator_local(&mut self, idx: usize, scope: &crate::config::CoordinatorScope) {
        let mut cmd = std::process::Command::new(
            std::env::current_exe().unwrap_or_else(|_| "clc".into()),
        );
        cmd.arg("coordinator-run");
        self.append_coordinator_args(&mut cmd, scope);

        cmd.env("CLC_API_PORT", "19100");
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

        let image = scope.docker_image.as_deref().unwrap_or("clc-worker:latest");
        let tunnel_port = 19200 + idx as u16;

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
        // Coordinator in Docker dispatches workers via API, not local process.
        start_cmd.push("--workspace".to_string());
        start_cmd.push("docker".to_string());
        if let Some(ref img) = scope.docker_image {
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
            api_port: 19100,
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
                    let image = c.scope.docker_image.as_deref().unwrap_or("clc-worker:latest").to_string();
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

        let tunnel_port = 19200 + self.coordinators.len() as u16 + self.workers.len() as u16;

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

        // Read the oauth token from the environment (set when clc up was invoked).
        let oauth_token = std::env::var("CLC_CLAUDE_CODE_OAUTH_TOKEN")
            .or_else(|_| std::env::var("CLAUDE_CODE_OAUTH_TOKEN"))
            .ok();

        let ws_config = WorkspaceConfig {
            agent_config: AgentConfig {
                model: model.to_string(),
                system_prompt: String::new(),
                initial_prompt: String::new(),
                extra_args: vec![],
            },
            tisket_id: tisket_id.to_string(),
            project_dir: self.project_dir.clone(),
            main_branch: self.main_branch.clone(),
        };

        let ssh_config = SSHWorkspaceConfig {
            workspace_config: ws_config,
            ca,
            api_port: 19100,
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
                eprintln!("supervisor: worker '{tisket_id}' started in Docker");
                self.workers.push(WorkerState {
                    tisket_id: tisket_id.to_string(),
                    coordinator_id: coordinator_id.to_string(),
                    model: model.to_string(),
                    _workspace: Some(workspace),
                });
            }
            Err(e) => {
                eprintln!("supervisor: docker start failed for worker '{tisket_id}': {e}");
                let _ = coord.set_status(tisket_id, clc_sdk::coordination::AgentStatus::Failed);
            }
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid.cast_signed()), None).is_ok()
}
