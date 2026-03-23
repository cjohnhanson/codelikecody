mod adapter;
mod admin;
mod cli;
mod config;
mod coordinate;
mod docs;
mod coordinator_mgmt;
mod dispatch;
mod done;
mod error;
mod event;
mod git;
mod git_add;
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
mod tisket;
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
        cli::Command::Coordinators { all } => cmd_coordinators(all),
        cli::Command::Coordinator { ref id, ref action } => cmd_coordinator(id, action),
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
