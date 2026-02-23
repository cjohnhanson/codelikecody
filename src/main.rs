mod adapter;
mod cli;
mod config;
mod error;
mod event;
mod git;
mod guard;
mod hook;
mod init;

use error::Error;

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
        cli::Command::Init => cmd_init(),
        cli::Command::Status => cmd_status(),
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
    if let Some(state) = git::detect(&cwd) {
        println!("branch: {}", state.branch);
        println!("is_main: {}", state.is_main);
        println!("is_worktree: {}", state.is_worktree);
    } else {
        println!("no git repository detected");
    }
    Ok(())
}

fn cmd_config(action: &cli::ConfigAction) -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    match action {
        cli::ConfigAction::Show => config::show(&project_dir),
    }
}

fn cmd_init() -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    init::init(&project_dir)?;
    eprintln!("initialized clc in {}", project_dir.display());
    Ok(())
}
