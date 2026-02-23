mod cli;
mod config;
mod error;
mod init;

use error::Error;

fn main() {
    let cli = <cli::Cli as clap::Parser>::parse();

    let result = match cli.command {
        cli::Command::Init => cmd_init(),
        cli::Command::Config { ref action } => cmd_config(action),
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
