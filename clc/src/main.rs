mod adapter;
mod cli;
mod config;
mod done;
mod error;
mod event;
mod git;
mod guard;
mod hook;
mod init;
mod missouri;
mod phase;
mod pickup;
mod tisket;

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
        cli::Command::Init { untracked } => cmd_init(untracked),
        cli::Command::Status { action: None } => cmd_status(),
        cli::Command::Status {
            action: Some(cli::StatusAction::Set { ref phase }),
        } => cmd_status_set(phase),
        cli::Command::Pickup { ref id } => cmd_pickup(id),
        cli::Command::Done => cmd_done(),
        cli::Command::Prime => cmd_prime(),
        cli::Command::Config { ref action } => cmd_config(action),
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

fn cmd_config(action: &cli::ConfigAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::ConfigAction::Show => config::show(&project_dir),
    }
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

fn cmd_init(untracked: bool) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    init::init(&project_dir, untracked)?;
    if untracked {
        eprintln!("initialized clc (untracked) in {}", project_dir.display());
    } else {
        eprintln!("initialized clc in {}", project_dir.display());
    }
    Ok(())
}
