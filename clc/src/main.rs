mod adapter;
mod admin;
mod belmont;
mod cli;
mod config;
mod coordination;
mod coordination_client;
mod docs;
mod coordinator_loop;
mod coordinator_mgmt;
mod dispatch;
mod done;
mod error;
mod event;
mod git;
mod git_add;
mod git_pack;
mod gix_ops;
mod guard;
mod home;
mod hook;
mod init;
mod integrate;
mod merge;
mod missouri;
mod permissions;
mod phase;
mod pickup;
mod review;
mod reviewer;
mod skills;
mod supervisor;
mod ssh_session;
mod ssh_workspace;
mod supervisor_api;
mod tisket;
mod tls;
mod topology;
mod zettel;
mod worker;
mod workflow;

use std::path::Path;

use clc_sdk::ClcTool;
use error::Error;

/// Check whether clc is running in untracked mode by reading the git
/// exclude file. Encapsulated so the detection logic can change without
/// affecting callers.
pub fn is_untracked(project_dir: &Path) -> bool {
    let exclude_path = project_dir.join(".git").join("info").join("exclude");
    std::fs::read_to_string(exclude_path)
        .map(|content| content.lines().any(|line| line.trim() == ".clc/"))
        .unwrap_or(false)
}

fn main() {
    sigpipe::reset();
    // Install rustls crypto provider for mTLS support.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = <cli::Cli as clap::Parser>::parse();

    if matches!(cli.command, cli::Command::Hook) {
        match hook::run() {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("clc: {e}");
                std::process::exit(e.exit_code());
            }
        }
    }

    let result = match cli.command {
        cli::Command::Init { untracked, force, user } => {
            if user {
                cmd_init_user()
            } else {
                cmd_init(untracked, force)
            }
        }
        cli::Command::Status { action: None } => cmd_status(),
        cli::Command::Status {
            action: Some(cli::StatusAction::Set { ref phase }),
        } => cmd_status_set(phase),
        cli::Command::Admin => cmd_admin(),
        cli::Command::Up { dry_run } => cmd_up(dry_run),
        cli::Command::CoordinatorRun {
            ref id,
            max_workers,
            ref model,
            ref project,
            ref label,
            ref exclude_label,
            ref auto_grant,
            ref always_escalate,
            poll_interval,
            ref workspace,
            ref docker_image,
            dry_run,
            ref filter,
            ref depends_on,
            ref grant_config,
            ref tisket,
            ref workflow,
        } => cmd_coordinator_run(
            id,
            max_workers,
            model,
            project.as_deref(),
            label.as_deref(),
            exclude_label.as_deref(),
            auto_grant,
            always_escalate,
            poll_interval,
            workspace,
            docker_image.as_deref(),
            dry_run,
            filter.as_deref(),
            depends_on.as_deref(),
            grant_config.as_deref(),
            tisket.as_deref(),
            workflow.as_deref(),
        ),
        cli::Command::Home => cmd_home(),
        cli::Command::Merge { ref id } => cmd_merge(id),
        cli::Command::Pickup { ref id } => cmd_pickup(id),
        cli::Command::Done => cmd_done(),
        cli::Command::Prime => cmd_prime(),
        cli::Command::Config { ref action } => cmd_config(action),
        cli::Command::Almanac { command } => cmd_almanac(command),
        cli::Command::Dispatch {
            ref id,
            ref model,
            ref coordinator_id,
        } => cmd_dispatch(id, model, coordinator_id.as_deref()),
        cli::Command::Workers {
            all,
            prune,
            stranded,
        } => cmd_workers(all, prune, stranded),
        cli::Command::Worker { ref id, ref action } => cmd_worker(id, action),
        cli::Command::Land { ref id } => cmd_land(id),
        cli::Command::Docs { topic, query } => cmd_docs(topic.as_deref(), query.as_deref()),
        cli::Command::Tisket { command } => cmd_tisket(command),
        cli::Command::Missouri { command } => cmd_missouri(command),
        cli::Command::Zettel { command } => cmd_zettel(command),
        cli::Command::Belmont { command } => cmd_belmont(command),
        cli::Command::Permissions { ref action } => cmd_permissions(action),
        cli::Command::Review { ref action } => cmd_review(action),
        cli::Command::Inbox { ref action } => cmd_inbox(action),
        cli::Command::Outbox { ref action } => cmd_outbox(action),
        cli::Command::Integrate { ref action } => cmd_integrate(action),
        cli::Command::Workspace { ref action } => cmd_workspace(action),
        cli::Command::Coordinators { all } => cmd_coordinators(all),
        cli::Command::Coordinator { ref id, ref action } => cmd_coordinator(id, action),
        cli::Command::Remind {
            seconds,
            message,
            repeat,
        } => cmd_remind(seconds, &message, repeat),
        cli::Command::Hook => unreachable!(),
    };

    if let Err(e) = result {
        if let Error::Block(msg) = &e {
            eprintln!("{msg}");
            std::process::exit(2);
        } else {
            eprintln!("clc: {e}");
            std::process::exit(e.exit_code());
        }
    }
}

fn cmd_status() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd).unwrap_or_default();
    let initialized = cwd.join(".clc").is_dir();

    let untracked = is_untracked(&cwd);
    println!("initialized: {initialized}");
    println!("untracked: {untracked}");
    println!("main_branch: {}", cfg.main_branch);

    if let Some(p) = phase::load_name(&cwd)? {
        println!("phase: {p}");
        if cfg.required_attempts > 1 {
            let attempts = phase::load_attempts(&cwd)?;
            println!("attempts: {attempts}/{}", cfg.required_attempts);
        }
    }

    let git_state = git::detect(&cwd, &cfg.main_branch, &cfg.admin_branch);
    if let Some(ref state) = git_state {
        println!("branch: {}", state.branch);
        println!("is_main: {}", state.is_main);
        println!("is_worktree: {}", state.is_worktree);
    } else {
        println!("no git repository detected");
    }

    let branch = git_state.as_ref().map(|s| s.branch.as_str());
    match tisket::detect(&cwd, branch) {
        Ok(state) => {
            println!("{}", state.status_basic());
            if let Some(ref issue) = state.current_issue {
                println!("tisket_issue: {}", issue.id);
                println!("tisket_title: {}", issue.title);
                println!("tisket_status: {}", issue.status);
            }
        }
        Err(e) => {
            println!("tisket: error ({e})");
        }
    }

    match topology::load(&cwd) {
        Ok(Some(topo)) => {
            println!(
                "topology: {} workspace(s), {} coordinator(s)",
                topo.workspaces.len(),
                topo.coordinators.len()
            );
        }
        Ok(None) => {}
        Err(e) => {
            println!("topology: error ({e})");
        }
    }

    match zettel::detect(&cwd) {
        Ok(state) => {
            println!("{}", state.status_basic());
        }
        Err(e) => {
            println!("zettel: error ({e})");
        }
    }

    match belmont::detect(&cwd) {
        Ok(state) if state.initialized => {
            println!("{}", state.status_basic());
        }
        Ok(_) => {}
        Err(e) => {
            println!("belmont: error ({e})");
        }
    }

    match missouri::run_tests(&cwd) {
        Ok(Some(summary)) => {
            println!("{}", summary.status_basic());
        }
        Ok(None) => {
            println!(
                "{}",
                missouri::MissouriState {
                    has_tests: false,
                    path_count: 0,
                    state_count: 0,
                }
                .status_basic()
            );
        }
        Err(e) => {
            println!("missouri: error ({e})");
        }
    }

    Ok(())
}

fn cmd_status_set(target: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd).unwrap_or_default();
    let wf_name = phase::load_workflow_name(&cwd).unwrap_or(None);
    let wf = wf_name
        .as_ref()
        .and_then(|name| cfg.workflows.get(name))
        .and_then(|def| workflow::Workflow::new(def).ok())
        .unwrap_or_else(workflow::Workflow::default_tdd);

    // Enforce test command at "green" phase boundary.
    if target == "green" {
        if let Some(ref cmd) = cfg.test_command {
            eprintln!("running test command: {cmd}");
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&cwd)
                .status()
                .map_err(|e| Error::NonBlocking(format!("test command failed to start: {e}")))?;
            if !status.success() {
                return Err(Error::NonBlocking(format!(
                    "cannot advance to 'green': test command failed (exit {})",
                    status.code().unwrap_or(-1)
                )));
            }
        }
    }

    // Try the transition. If blocked by a review gate, poll until approved.
    match phase::set_with_workflow(&cwd, target, cfg.required_attempts, &wf) {
        Ok(()) => Ok(()),
        Err(e) if e.is_review_required() => {
            eprintln!("{e}");
            eprintln!("Waiting for review approval...");
            // Poll every 15 seconds until the review gate opens.
            for _ in 0..120 {
                std::thread::sleep(std::time::Duration::from_secs(15));
                match phase::set_with_workflow(&cwd, target, cfg.required_attempts, &wf) {
                    Ok(()) => {
                        eprintln!("Review approved — phase advanced to '{target}'");
                        return Ok(());
                    }
                    Err(ref retry_err) if retry_err.is_review_required() => {
                        continue;
                    }
                    Err(other) => return Err(other),
                }
            }
            Err(Error::NonBlocking(format!(
                "review gate timeout after 30 minutes for transition to '{target}'"
            )))
        }
        Err(e) => Err(e),
    }
}

fn cmd_up(dry_run: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();

    // Prefer clc.yaml (topology format) over clc.yml supervisor block.
    let (sup_config, config_source) = if let Some(topo) = topology::load(&project_dir)? {
        (topo.to_supervisor_config(), "clc.yaml")
    } else {
        (cfg.supervisor.clone(), "clc.yml")
    };

    if sup_config.coordinators.is_empty() {
        return Err(Error::NonBlocking(
            "no coordinators configured — add coordinators to clc.yaml or clc.yml".into(),
        ));
    }

    // Validate workflow agents resolve to .clc/reviewers/ files.
    // Validate that reviewer agent files exist for all review-gated transitions.
    for (wf_name, wf_def) in &sup_config.workflows {
        let wf = crate::workflow::Workflow::new(wf_def).map_err(|e| {
            Error::NonBlocking(format!("workflow '{wf_name}': {e}"))
        })?;
        for phase_name in wf.phase_names() {
            for agent_name in wf.reviewers_from(phase_name) {
                reviewer::resolve(&project_dir, &agent_name)?;
            }
        }
    }

    if dry_run {
        println!("config: {config_source}");
        println!("poll_interval: {}s", sup_config.poll_interval);
        println!("coordinators: {}", sup_config.coordinators.len());
        for c in &sup_config.coordinators {
            print!("  {} (model={}, workspace={}, max_workers={})", c.id, c.model, c.workspace, c.max_workers);
            if let Some(ref label) = c.label {
                print!(", label={label}");
            }
            if let Some(ref project) = c.project {
                print!(", project={project}");
            }
            if let Some(ref wf) = c.workflow {
                print!(", workflow={wf}");
            }
            println!();
        }
        return Ok(());
    }

    let mut sup = supervisor::Supervisor::new(
        &project_dir,
        &cfg.main_branch,
        &cfg.admin_branch,
        &sup_config,
    );
    sup.run()
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn cmd_coordinator_run(
    id: &str,
    max_workers: usize,
    model: &str,
    project: Option<&str>,
    label: Option<&str>,
    exclude_label: Option<&str>,
    auto_grant: &[String],
    always_escalate: &[String],
    poll_interval: u64,
    workspace: &str,
    docker_image: Option<&str>,
    dry_run: bool,
    filter: Option<&str>,
    depends_on: Option<&str>,
    grant_config: Option<&str>,
    tisket: Option<&str>,
    workflow: Option<&str>,
) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();

    // Parse --filter into label/project/exclude-label overrides.
    let (filter_label, filter_project, filter_exclude) = parse_filter(filter);
    let effective_label = label.map(str::to_string).or(filter_label);
    let effective_project = project.map(str::to_string).or(filter_project);
    let effective_exclude = exclude_label.map(str::to_string).or(filter_exclude);

    let scope = config::CoordinatorScope {
        id: id.to_string(),
        project: effective_project,
        label: effective_label,
        exclude_label: effective_exclude,
        max_workers,
        model: model.to_string(),
        workspace: workspace.to_string(),
        image: docker_image.map(str::to_string),
        auto_grant: auto_grant.to_vec(),
        always_escalate: always_escalate.to_vec(),
        workflow: workflow.map(str::to_string),
    };

    // Validate grant-config file if provided.
    if let Some(path) = grant_config {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::NonBlocking(format!("grant-config '{path}': {e}"))
        })?;
        serde_yml::from_str::<serde_json::Value>(&content).map_err(|e| {
            Error::NonBlocking(format!("grant-config '{path}': invalid YAML: {e}"))
        })?;
    }

    if dry_run {
        return coordinator_loop::dry_run(
            &project_dir,
            &cfg.main_branch,
            &cfg.admin_branch,
            &scope,
            depends_on,
            tisket,
        );
    }

    coordinator_loop::run(
        &project_dir,
        &cfg.main_branch,
        &cfg.admin_branch,
        &scope,
        &cfg.worker.permissions.default,
        &cfg.worker.permissions.deny,
        std::time::Duration::from_secs(poll_interval),
    )
}

/// Parse a combined filter string like "label:feature,project:v0.1.0".
fn parse_filter(filter: Option<&str>) -> (Option<String>, Option<String>, Option<String>) {
    let Some(f) = filter else {
        return (None, None, None);
    };
    let mut label = None;
    let mut project = None;
    let mut exclude = None;
    for part in f.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("label:") {
            label = Some(v.to_string());
        } else if let Some(v) = part.strip_prefix("project:") {
            project = Some(v.to_string());
        } else if let Some(v) = part.strip_prefix("exclude-label:") {
            exclude = Some(v.to_string());
        } else if let Some(v) = part.strip_prefix("status:") {
            // status:todo is implicit — all pickable tiskets are todo.
            let _ = v;
        }
    }
    (label, project, exclude)
}

fn cmd_config(action: &cli::ConfigAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::ConfigAction::Show => config::show(&project_dir),
    }
}

fn cmd_almanac(command: ::almanac::cli::Command) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let mut cfg = config::load(&project_dir).unwrap_or_default();
    if let Some(ref uc) = config::load_user_config().unwrap_or(None) {
        config::merge_user_config(&mut cfg, uc);
    }
    ::almanac::cli::run_command(&project_dir, &cfg.skills, command)
        .map_err(|e: ::almanac::Error| Error::NonBlocking(e.to_string()))
}

fn cmd_merge(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    merge::merge(&project_dir, id, &cfg.main_branch, &cfg.admin_branch)?;
    eprintln!("merged '{id}' into trunk");
    Ok(())
}

fn cmd_home() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let path = home::home(&cwd)?;
    println!("{}", path.display());
    Ok(())
}

fn cmd_admin() -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    admin::admin(&project_dir, &cfg.main_branch, &cfg.admin_branch)?;
    eprintln!("admin worktree ready at .worktrees/{}", cfg.admin_branch);
    Ok(())
}

fn cmd_pickup(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    pickup::pickup(&project_dir, id, &cfg.main_branch, &cfg.admin_branch, None)?;
    eprintln!("picked up '{id}' — worktree at .worktrees/{id}");
    Ok(())
}

fn cmd_workspace(action: &cli::WorkspaceAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    match action {
        cli::WorkspaceAction::Init => {
            // Create project dir structure and worker stdio infrastructure.
            let worker_dir = cwd.join(".clc").join("worker");
            std::fs::create_dir_all(&worker_dir)?;

            // Create named pipe for stdin.
            let pipe_path = worker_dir.join("stdin.pipe");
            if pipe_path.exists() {
                std::fs::remove_file(&pipe_path)?;
            }
            nix::unistd::mkfifo(
                &pipe_path,
                nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
            )
            .map_err(|e| Error::NonBlocking(format!("mkfifo: {e}")))?;

            // Create stdout/stderr files.
            std::fs::File::create(worker_dir.join("stdout.jsonl"))?;
            std::fs::File::create(worker_dir.join("stderr.log"))?;

            // Initialize clc hooks.
            init::init(&cwd, false, true)?;

            // Seed baseline permissions so the worker can run clc/tisket/cargo etc.
            let cfg = config::load(&cwd).unwrap_or_default();
            permissions::seed_defaults(&cwd, &cfg.worker.permissions.default, &cfg.worker.permissions.deny)?;

            eprintln!("workspace initialized at {}", cwd.display());
            Ok(())
        }
        cli::WorkspaceAction::Export { branch, output } => {
            let pack = git_pack::create_pack(&cwd, branch)?;
            let b64 = crate::ssh_workspace::base64_encode(&pack.pack);
            let refs: Vec<serde_json::Value> = pack
                .refs
                .iter()
                .map(|(oid, name)| serde_json::json!([oid, name]))
                .collect();
            let envelope = serde_json::json!({
                "pack": b64,
                "refs": refs,
                "branch": branch,
            });
            let json = serde_json::to_string(&envelope)?;
            if let Some(path) = output {
                std::fs::write(path, &json)
                    .map_err(|e| Error::NonBlocking(format!("write export: {e}")))?;
            } else {
                println!("{json}");
            }
            Ok(())
        }
        cli::WorkspaceAction::Start {
            model,
            branch,
            api_url,
            oauth_token,
        } => {
            use clc_sdk::agent::{Agent, AgentConfig, ClaudeCodeAgent};
            use std::process::Stdio;

            let worker_dir = cwd.join(".clc").join("worker");
            let pid_path = worker_dir.join("pid");
            let stdout_path = worker_dir.join("stdout.jsonl");
            let stderr_path = worker_dir.join("stderr.log");
            let stdin_pipe_path = worker_dir.join("stdin.pipe");

            // Create and checkout the tisket branch so the worker starts
            // on the right branch (not main). Hooks enforce branch-based
            // phase constraints.
            if git::current_branch(&cwd).as_deref() != Some(branch) {
                crate::gix_ops::create_branch(&cwd, branch)?;
                crate::gix_ops::checkout_branch(&cwd, branch)?;

                // Set git identity for the worker (required for commits).
                let git_config = cwd.join(".git").join("config");
                let mut config_content = std::fs::read_to_string(&git_config).unwrap_or_default();
                if !config_content.contains("[user]") {
                    config_content.push_str("\n[user]\n\tname = clc-worker\n\temail = worker@clc.local\n");
                    let _ = std::fs::write(&git_config, config_content);
                }
                // Set initial phase — route through API if available.
                // Retry briefly: the supervisor API may still be starting.
                if let Some(url) = &api_url {
                    let mut set = false;
                    for attempt in 0..5 {
                        if attempt > 0 {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                        let default_initial = crate::workflow::Workflow::default_tdd().initial_phase().to_string();
                        if crate::phase::init_phase_via_api(url, branch, &default_initial, None).is_ok() {
                            set = true;
                            break;
                        }
                    }
                    if !set {
                        eprintln!("warning: could not set initial phase via API (may already exist)");
                    }
                } else {
                    let default_initial = crate::workflow::Workflow::default_tdd().initial_phase().to_string();
                    crate::phase::init_phase_with_workflow(&cwd, &default_initial, None)?;
                }
            }

            // Build prompts.
            let system_prompt = dispatch::build_system_prompt(branch, None);
            let initial_prompt = dispatch::build_worker_prompt_from_dir(&cwd, branch)?;

            let agent = ClaudeCodeAgent::new();
            let config = AgentConfig {
                model: model.clone(),
                system_prompt,
                initial_prompt: initial_prompt.clone(),
                extra_args: vec![],
                allowed_tools: vec![],
            };

            let mut cmd = agent
                .build_start_command(&config, &cwd)
                .map_err(|e| Error::NonBlocking(format!("build command: {e}")))?;

            // Set env vars via Command::env — no shell export.
            if let Some(url) = api_url {
                cmd.env("CLC_API_URL", url);
            }
            if let Some(token) = oauth_token {
                cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token);
            }

            // Wire mTLS cert env vars if certs were deployed by the SSH workspace.
            let cert_path = "/tmp/workspace-cert.pem";
            let key_path = "/tmp/workspace-key.pem";
            let ca_path = "/tmp/ca-cert.pem";
            if std::path::Path::new(cert_path).exists() {
                cmd.env("CLC_API_CERT", cert_path);
                cmd.env("CLC_API_KEY", key_path);
                cmd.env("CLC_API_CA", ca_path);
            }

            // Wire stdio to pipes/files — same as spawn_agent_process.
            let stdin_file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&stdin_pipe_path)?;
            let stdout_file = std::fs::File::create(&stdout_path)?;
            let stderr_file = std::fs::File::create(&stderr_path)?;

            cmd.stdin(Stdio::from(stdin_file));
            cmd.stdout(Stdio::from(stdout_file));
            cmd.stderr(Stdio::from(stderr_file));

            let child = cmd
                .spawn()
                .map_err(|e| Error::NonBlocking(format!("spawn: {e}")))?;

            let pid = child.id();
            std::fs::write(&pid_path, pid.to_string())?;

            // Send initial prompt to the pipe.
            dispatch::send_prompt(&stdin_pipe_path, &initial_prompt)?;

            eprintln!("agent started (pid {pid})");
            Ok(())
        }
        cli::WorkspaceAction::WriteFile { path } => {
            use std::io::Read;
            let mut data = Vec::new();
            std::io::stdin()
                .read_to_end(&mut data)
                .map_err(|e| Error::NonBlocking(format!("read stdin: {e}")))?;
            if let Some(parent) = std::path::Path::new(path.as_str()).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, &data)
                .map_err(|e| Error::NonBlocking(format!("write file: {e}")))?;
            Ok(())
        }
        cli::WorkspaceAction::Receive {
            bundle,
            branch,
            stdin,
            dir,
            pack_file,
            refs_file,
        } => {
            let target_dir = dir
                .as_ref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| cwd.clone());

            let (pack_data, refs) = if let (Some(pf), Some(rf)) = (pack_file, refs_file) {
                // File-based transfer: binary pack + JSON refs.
                let pack = std::fs::read(pf)
                    .map_err(|e| Error::NonBlocking(format!("read pack file: {e}")))?;
                let refs_json = std::fs::read_to_string(rf)
                    .map_err(|e| Error::NonBlocking(format!("read refs file: {e}")))?;
                let refs: Vec<(String, String)> = serde_json::from_str::<Vec<Vec<String>>>(&refs_json)
                    .map_err(|e| Error::NonBlocking(format!("parse refs: {e}")))?
                    .into_iter()
                    .filter_map(|r| Some((r.first()?.clone(), r.get(1)?.clone())))
                    .collect();
                (pack, refs)
            } else {
                // Legacy: JSON envelope from stdin or file.
                use std::io::Read;
                let mut data = Vec::new();

                if *stdin {
                    std::io::stdin()
                        .read_to_end(&mut data)
                        .map_err(|e| Error::NonBlocking(format!("read stdin: {e}")))?;
                } else {
                    let path = bundle
                        .as_ref()
                        .ok_or_else(|| Error::NonBlocking("path required when not using --stdin".into()))?;
                    data = std::fs::read(path)
                        .map_err(|e| Error::NonBlocking(format!("read file: {e}")))?;
                }

                let envelope: serde_json::Value = serde_json::from_slice(&data)
                    .map_err(|e| Error::NonBlocking(format!("parse envelope: {e}")))?;

                let pack_b64 = envelope["pack"]
                    .as_str()
                    .ok_or_else(|| Error::NonBlocking("missing pack field".into()))?;
                let pack = base64_decode(pack_b64)
                    .map_err(|e| Error::NonBlocking(format!("decode pack: {e}")))?;

                let refs: Vec<(String, String)> = envelope["refs"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|r| {
                        let arr = r.as_array()?;
                        Some((arr.first()?.as_str()?.to_string(), arr.get(1)?.as_str()?.to_string()))
                    })
                    .collect();

                (pack, refs)
            };

            git_pack::receive_pack(&pack_data, &refs, &target_dir, branch)?;

            eprintln!("received repo, checked out branch '{branch}'");
            Ok(())
        }
    }
}

pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const DECODE: [u8; 256] = {
        let mut t = [255u8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        t
    };

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b'=').collect();

    for chunk in bytes.chunks(4) {
        let a = DECODE[chunk[0] as usize] as u32;
        let b = if chunk.len() > 1 { DECODE[chunk[1] as usize] as u32 } else { 0 };
        let c = if chunk.len() > 2 { DECODE[chunk[2] as usize] as u32 } else { 0 };
        let d = if chunk.len() > 3 { DECODE[chunk[3] as usize] as u32 } else { 0 };

        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        if chunk.len() > 2 { out.push((n >> 8) as u8); }
        if chunk.len() > 3 { out.push(n as u8); }
    }

    Ok(out)
}

fn cmd_done() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd).unwrap_or_default();
    done::done(&cwd, &cfg.main_branch, &cfg.admin_branch)?;
    eprintln!("done — work finalized");
    Ok(())
}

fn cmd_prime() -> Result<(), Error> {
    let text = hook::prime_text()?;
    print!("{text}");
    Ok(())
}

fn cmd_remind(seconds: u64, message: &str, repeat: u32) -> Result<(), Error> {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    println!("{message}");
    if repeat > 0 {
        let next = repeat - 1;
        let escaped = message.replace('\'', "'\\''");
        println!(
            "\nThis reminder has {next} repetition{} remaining. \
             Run this command to continue:\n\n\
             clc remind {seconds} '{escaped}' --repeat {next}",
            if next == 1 { "" } else { "s" }
        );
    }
    Ok(())
}

fn cmd_docs(topic: Option<&str>, query: Option<&str>) -> Result<(), Error> {
    match topic {
        None | Some("list") => {
            docs::list();
            Ok(())
        }
        Some("search") => {
            let q = query.unwrap_or("");
            if q.is_empty() {
                return Err(Error::NonBlocking(
                    "usage: clc docs search <query>".to_string(),
                ));
            }
            docs::search(q);
            Ok(())
        }
        Some(identifier) => {
            if docs::show(identifier) {
                Ok(())
            } else {
                eprintln!("unknown doc: {identifier}");
                eprintln!();
                docs::list();
                Err(Error::NonBlocking(format!("doc '{identifier}' not found")))
            }
        }
    }
}

fn cmd_tisket(command: ::tisket::cli::Command) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let root = camino::Utf8PathBuf::try_from(cwd)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 path: {e}")))?;
    ::tisket::cli::run_command(&root, command)
        .map_err(|e: ::tisket::Error| Error::NonBlocking(e.to_string()))
}

fn cmd_zettel(command: ::zettel::cli::Command) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let root = camino::Utf8PathBuf::try_from(cwd)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 path: {e}")))?;
    let args = ::zettel::cli::Args {
        root,
        command,
    };
    ::zettel::cli::run(args).map_err(|e: ::zettel::Error| Error::NonBlocking(e.to_string()))
}

fn cmd_belmont(command: ::belmont::cli::Command) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let root = camino::Utf8PathBuf::try_from(cwd)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 path: {e}")))?;
    let args = ::belmont::cli::Args {
        root,
        command,
    };
    ::belmont::cli::run(args).map_err(|e: ::belmont::Error| Error::NonBlocking(e.to_string()))
}

fn cmd_missouri(command: ::missouri::cli::Command) -> Result<(), Error> {
    let config_dir = ".missouri";
    match ::missouri::cli::run_command(config_dir, command) {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::Block("missouri: tests failed".to_string())),
        Err(e) => Err(Error::NonBlocking(format!("{e}"))),
    }
}

fn cmd_dispatch(id: &str, model: &str, coordinator_id: Option<&str>) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    dispatch::dispatch(
        &project_dir,
        id,
        &cfg.main_branch,
        &cfg.admin_branch,
        model,
        &cfg.worker.permissions.default,
        &cfg.worker.permissions.deny,
        coordinator_id,
    )
}

fn cmd_workers(all: bool, prune: bool, stranded: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    if prune {
        worker::prune_workers(&project_dir)
    } else if stranded {
        worker::list_stranded(&project_dir)
    } else {
        worker::list_workers(&project_dir, all)
    }
}

fn cmd_worker(id: &str, action: &cli::WorkerAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::WorkerAction::Check => worker::check(&project_dir, id),
        cli::WorkerAction::Log { lines } => worker::log(&project_dir, id, *lines),
        cli::WorkerAction::Send { message } => worker::send(&project_dir, id, message),
        cli::WorkerAction::Stop => worker::stop(&project_dir, id),
        cli::WorkerAction::Resume => worker::resume(&project_dir, id),
        cli::WorkerAction::Supervise { max_resumes } => {
            worker::supervise(&project_dir, id, *max_resumes)
        }
        cli::WorkerAction::Raw { lines } => worker::raw(&project_dir, id, *lines),
        cli::WorkerAction::Recover => {
            let cfg = config::load(&project_dir).unwrap_or_default();
            worker::recover(&project_dir, id, &cfg.main_branch, &cfg.admin_branch)
        }
    }
}

fn cmd_land(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    worker::land(&project_dir, id, &cfg.main_branch, &cfg.admin_branch)
}

fn cmd_permissions(action: &cli::PermissionsAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    // Request uses cwd directly — workers run from their own worktree.
    if let cli::PermissionsAction::Request { description } = action {
        return permissions::request(&cwd, description);
    }
    // All other commands resolve the project root so they work from any
    // worktree (admin, worker, or trunk).
    let project_dir = home::home(&cwd)?;
    match action {
        cli::PermissionsAction::Request { .. } => unreachable!(),
        cli::PermissionsAction::Grant {
            worker_id,
            permission,
        } => permissions::grant(&project_dir, worker_id, permission),
        cli::PermissionsAction::List => permissions::list(&project_dir),
        cli::PermissionsAction::Escalate {
            worker_id,
            description,
        } => permissions::escalate(&project_dir, worker_id, description),
        cli::PermissionsAction::Inbox => permissions::inbox(&project_dir),
        cli::PermissionsAction::Deny { worker_id, reason } => {
            permissions::deny(&project_dir, worker_id, reason)
        }
    }
}

fn cmd_review(action: &cli::ReviewAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    match action {
        cli::ReviewAction::Request { review_type } => review::request(&cwd, review_type),
        cli::ReviewAction::Approve { comments } => review::approve(&cwd, comments),
        cli::ReviewAction::RequestChanges { comments } => review::request_changes(&cwd, comments),
    }
}

fn cmd_inbox(action: &cli::InboxAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    match action {
        cli::InboxAction::Poll => {
            use clc_sdk::inbox::Inbox as _;
            let inbox_dir = cwd.join(".clc").join("inbox");
            if !inbox_dir.exists() {
                return Ok(());
            }
            let mut inbox = clc_sdk::inbox::FolderInbox::new(&inbox_dir);
            let items = inbox
                .poll()
                .map_err(|e| Error::NonBlocking(format!("inbox: {e}")))?;
            for item in &items {
                println!("{}\t{}", item.source(), item.content());
            }
        }
    }
    Ok(())
}

fn cmd_outbox(action: &cli::OutboxAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    match action {
        cli::OutboxAction::Write { name } => {
            use clc_sdk::outbox::Outbox as _;
            use std::io::Read as _;
            let mut content = String::new();
            std::io::stdin()
                .read_to_string(&mut content)
                .map_err(|e| Error::NonBlocking(format!("failed to read stdin: {e}")))?;
            let outbox_dir = cwd.join(".clc").join("outbox");
            let outbox = clc_sdk::outbox::FolderOutbox::new(outbox_dir);
            outbox
                .send(clc_sdk::outbox::OutboxItem {
                    name: name.clone(),
                    content,
                })
                .map_err(|e| Error::NonBlocking(format!("outbox: {e}")))?;
        }
    }
    Ok(())
}

fn cmd_integrate(action: &cli::IntegrateAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    match action {
        cli::IntegrateAction::Create { name } => {
            integrate::create(&project_dir, name, &cfg.main_branch)?;
            eprintln!("created integration branch integrate/{name}");
        }
        cli::IntegrateAction::Merge { branch } => {
            integrate::merge(&project_dir, branch)?;
            eprintln!("merged '{branch}' into integration branch");
        }
        cli::IntegrateAction::Land => {
            integrate::land(&project_dir, &cfg.main_branch)?;
            eprintln!("landed integration branch onto main");
        }
    }
    Ok(())
}

fn cmd_coordinators(all: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    coordinator_mgmt::list_coordinators(&project_dir, all)
}

fn cmd_coordinator(id: &str, action: &cli::CoordinatorAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::CoordinatorAction::Check => coordinator_mgmt::check(&project_dir, id),
        cli::CoordinatorAction::Log { lines } => coordinator_mgmt::log(&project_dir, id, *lines),
        cli::CoordinatorAction::Send { message } => {
            coordinator_mgmt::send(&project_dir, id, message)
        }
        cli::CoordinatorAction::Stop => coordinator_mgmt::stop(&project_dir, id),
        cli::CoordinatorAction::Land => {
            let cfg = config::load(&project_dir).unwrap_or_default();
            coordinator_mgmt::land(&project_dir, id, &cfg.main_branch)
        }
    }
}

fn cmd_init(untracked: bool, force: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    init::init(&project_dir, untracked, force)?;
    if untracked {
        eprintln!("initialized clc (untracked) in {}", project_dir.display());
    } else {
        eprintln!("initialized clc in {}", project_dir.display());
    }
    Ok(())
}

fn cmd_init_user() -> Result<(), Error> {
    init::init_user()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_subprocess_git_calls() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&src_dir).expect("read src/") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                let contents = std::fs::read_to_string(&path).expect("read file");
                assert!(
                    !contents.contains("Command::new(\"git\")"),
                    "found Command::new(\"git\") in {} — use gix_ops instead",
                    path.display()
                );
                assert!(
                    !contents.contains("Command::new(\"git\""),
                    "found Command::new(\"git\" in {} — use gix_ops instead",
                    path.display()
                );
            }
        }
    }
}
