mod adapter;
mod admin;
mod cli;
mod config;
mod coordinate;
mod dispatch;
mod done;
mod error;
mod event;
mod git;
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
mod tisket;
mod worker;

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
        } => cmd_coordinate(model, tisket.as_deref()),
        cli::Command::Home => cmd_home(),
        cli::Command::Merge { ref id } => cmd_merge(id),
        cli::Command::Pickup { ref id } => cmd_pickup(id),
        cli::Command::Done => cmd_done(),
        cli::Command::Prime => cmd_prime(),
        cli::Command::Config { ref action } => cmd_config(action),
        cli::Command::Dispatch { ref id, ref model } => cmd_dispatch(id, model),
        cli::Command::Workers { all, prune } => cmd_workers(all, prune),
        cli::Command::Worker { ref id, ref action } => cmd_worker(id, action),
        cli::Command::Land { ref id } => cmd_land(id),
        cli::Command::Tisket { command } => cmd_tisket(command),
        cli::Command::Missouri { command } => cmd_missouri(command),
        cli::Command::Permissions { ref action } => cmd_permissions(action),
        cli::Command::Integrate { ref action } => cmd_integrate(action),
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

    let git_state = git::detect(&cwd, &cfg.main_branch);
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

fn cmd_coordinate(model: &str, tisket: Option<&str>) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    coordinate::coordinate(
        &project_dir,
        &cfg.main_branch,
        model,
        tisket,
        &cfg.permissions.allow,
    )
}

fn cmd_config(action: &cli::ConfigAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::ConfigAction::Show => config::show(&project_dir),
    }
}

fn cmd_merge(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    merge::merge(&project_dir, id, &cfg.main_branch)?;
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
    admin::admin(&project_dir, &cfg.main_branch)?;
    eprintln!("admin worktree ready at .worktrees/clc-admin");
    Ok(())
}

fn cmd_pickup(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    pickup::pickup(&project_dir, id, &cfg.main_branch)?;
    eprintln!("picked up '{id}' — worktree at .worktrees/{id}");
    Ok(())
}

fn cmd_done() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd).unwrap_or_default();
    done::done(&cwd, &cfg.main_branch)?;
    eprintln!("done — work finalized");
    Ok(())
}

fn cmd_prime() -> Result<(), Error> {
    let text = hook::prime_text()?;
    print!("{text}");
    Ok(())
}

fn cmd_tisket(command: ::tisket::cli::Command) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let root = camino::Utf8PathBuf::try_from(cwd)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 path: {e}")))?;
    ::tisket::cli::run_command(&root, command)
        .map_err(|e: ::tisket::Error| Error::NonBlocking(e.to_string()))
}

fn cmd_missouri(command: ::missouri::cli::Command) -> Result<(), Error> {
    let config_dir = ".missouri";
    match ::missouri::cli::run_command(config_dir, command) {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::Block("missouri: tests failed".to_string())),
        Err(e) => Err(Error::NonBlocking(format!("{e}"))),
    }
}

fn cmd_dispatch(id: &str, model: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    dispatch::dispatch(
        &project_dir,
        id,
        &cfg.main_branch,
        model,
        &cfg.permissions.allow,
    )
}

fn cmd_workers(all: bool, prune: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    if prune {
        worker::prune_workers(&project_dir)
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
    }
}

fn cmd_land(id: &str) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    let cfg = config::load(&project_dir).unwrap_or_default();
    worker::land(&project_dir, id, &cfg.main_branch)
}

fn cmd_permissions(action: &cli::PermissionsAction) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    match action {
        cli::PermissionsAction::Request { description } => permissions::request(&cwd, description),
        cli::PermissionsAction::Grant {
            worker_id,
            permission,
        } => permissions::grant(&cwd, worker_id, permission),
        cli::PermissionsAction::List => permissions::list(&cwd),
        cli::PermissionsAction::Escalate {
            worker_id,
            description,
        } => permissions::escalate(&cwd, worker_id, description),
        cli::PermissionsAction::Inbox => permissions::inbox(&cwd),
    }
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
