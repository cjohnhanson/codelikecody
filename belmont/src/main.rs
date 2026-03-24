use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    sigpipe::reset();

    let args = belmont::cli::Args::parse();

    match belmont::cli::run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
