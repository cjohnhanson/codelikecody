mod adapter;
mod admin;
mod cli;
mod config;
mod coordinate;
mod coordination;
mod coordination_client;
mod docs;
mod coordinator_loop;
mod coordinator_mgmt;
mod dispatch;
mod docker_workspace;
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
mod workspace;

use std::path::Path;

use clc_sdk::ClcTool;
use error::Error;

fn is_untracked(project_dir: &Path) -> bool {
    let state_path = project_dir.join(".clc").join("state");
    std::fs::read_to_string(state_path)
        .map(|content| content.lines().any(|line| line.trim() == "untracked: true"))
        .unwrap_or(false)
}

fn main() {
    sigpipe::reset();
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
        cli::Command::Init { untracked, force } => cmd_init(untracked, force),
        cli::Command::Status { action: None } => cmd_status(),
        cli::Command::Status {
            action: Some(cli::StatusAction::Set { ref phase }),
        } => cmd_status_set(phase),
        cli::Command::Admin => cmd_admin(),
        cli::Command::Up => cmd_up(),
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
        ),
        cli::Command::Coordinate {
            ref model,
            ref tisket,
            ref label,
            ref exclude_label,
            ref project,
            ref depends_on,
            ref filter,
            dry_run,
            ref id,
            ref auto_grant,
            escalate_all,
            ref grant_config,
        } => {
            let filters = coordinate::CoordinateFilters {
                tisket: tisket.as_deref(),
                label: label.as_deref(),
                exclude_label: exclude_label.as_deref(),
                project: project.as_deref(),
                depends_on: depends_on.as_deref(),
                filter: filter.as_deref(),
                dry_run,
                coordinator_id: id.as_deref(),
                auto_grant,
                escalate_all,
                grant_config: grant_config.as_deref(),
            };
            cmd_coordinate(model, &filters)
        }
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
        cli::Command::Permissions { ref action } => cmd_permissions(action),
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

    if let Some(p) = phase::load(&cwd)? {
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
    phase::set(&cwd, target, cfg.required_attempts)
}

fn cmd_up() -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();

    if cfg.supervisor.coordinators.is_empty() {
        return Err(Error::NonBlocking(
            "no coordinator scopes configured in clc.yml — add [[supervisor.coordinators]] sections"
                .into(),
        ));
    }

    let mut sup = supervisor::Supervisor::new(
        &project_dir,
        &cfg.main_branch,
        &cfg.admin_branch,
        &cfg.supervisor,
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
) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();

    let ws_type = match workspace {
        "docker" => config::WorkspaceType::Docker,
        _ => config::WorkspaceType::Worktree,
    };

    let scope = config::CoordinatorScope {
        id: id.to_string(),
        project: project.map(str::to_string),
        label: label.map(str::to_string),
        exclude_label: exclude_label.map(str::to_string),
        max_workers,
        model: model.to_string(),
        workspace: ws_type,
        docker_image: docker_image.map(str::to_string),
        auto_grant: auto_grant.to_vec(),
        always_escalate: always_escalate.to_vec(),
    };

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

fn cmd_coordinate(model: &str, filters: &coordinate::CoordinateFilters<'_>) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    coordinate::coordinate(
        &project_dir,
        &cfg.main_branch,
        &cfg.admin_branch,
        model,
        filters,
        &cfg.worker.permissions.default,
        &cfg.worker.permissions.deny,
        &cfg.coordinator,
    )
}

fn cmd_config(action: &cli::ConfigAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::ConfigAction::Show => config::show(&project_dir),
    }
}

fn cmd_almanac(command: ::almanac::cli::Command) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
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

            eprintln!("workspace initialized at {}", cwd.display());
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
        } => {
            let target_dir = dir
                .as_ref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| cwd.clone());

            // Read pack data + refs from stdin (JSON envelope).
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

            // Parse JSON envelope: { "pack": base64, "refs": [["oid", "refname"], ...] }
            let envelope: serde_json::Value = serde_json::from_slice(&data)
                .map_err(|e| Error::NonBlocking(format!("parse envelope: {e}")))?;

            let pack_b64 = envelope["pack"]
                .as_str()
                .ok_or_else(|| Error::NonBlocking("missing pack field".into()))?;
            let pack_data = base64_decode(pack_b64)
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

            git_pack::receive_pack(&pack_data, &refs, &target_dir, branch)?;

            eprintln!("received repo, checked out branch '{branch}'");
            Ok(())
        }
    }
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
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
