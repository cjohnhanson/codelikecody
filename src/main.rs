mod cli;
mod error;
mod init;

use error::Error;

fn main() {
    let cli = <cli::Cli as clap::Parser>::parse();

    let result = match cli.command {
        cli::Command::Init => cmd_init(),
    };

    if let Err(e) = result {
        match &e {
            Error::Block(msg) => {
                eprintln!("{msg}");
                std::process::exit(2);
            }
            _ => {
                eprintln!("clc: {e}");
                std::process::exit(e.exit_code());
            }
        }
    }
}

fn cmd_init() -> Result<(), Error> {
    let project_dir = std::env::current_dir()?;
    init::init(&project_dir)?;
    eprintln!("initialized clc in {}", project_dir.display());
    Ok(())
}
