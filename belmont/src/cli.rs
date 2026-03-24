use camino::Utf8PathBuf;
use clap::Parser;

use crate::config::BelmontConfig;
use crate::error::Result;
use crate::registry::SecretRegistry;
use crate::runner;
use crate::scrub::Scrubber;

#[derive(Parser)]
#[command(
    name = "belmont",
    version,
    about = "Secrets management for coding agents",
    max_term_width = 98
)]
pub struct Args {
    /// Root directory of the project (default: current directory)
    #[arg(long, global = true, default_value = ".")]
    pub root: Utf8PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    /// Initialize belmont in the current project
    Init,
    /// List declared secret references (never values)
    List,
    /// Verify all secrets are resolvable
    Check,
    /// Run a command with secrets injected and output scrubbed
    Run(RunArgs),
}

#[derive(Parser)]
pub struct RunArgs {
    /// Command and arguments to execute
    #[arg(trailing_var_arg = true, required = true)]
    pub command: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Init => cmd_init(&args.root),
        Command::List => cmd_list(&args.root),
        Command::Check => cmd_check(&args.root),
        Command::Run(run_args) => cmd_run(&args.root, &run_args),
    }
}

fn cmd_init(root: &Utf8PathBuf) -> Result<()> {
    BelmontConfig::init(root.as_ref())?;
    eprintln!("initialized belmont.yml");
    Ok(())
}

fn cmd_list(root: &Utf8PathBuf) -> Result<()> {
    let config = BelmontConfig::load(root.as_ref())?;
    if config.secrets.is_empty() {
        eprintln!("no secrets declared");
        return Ok(());
    }
    for name in config.secrets.keys() {
        println!("belmont://{name}");
    }
    Ok(())
}

fn cmd_check(root: &Utf8PathBuf) -> Result<()> {
    let config = BelmontConfig::load(root.as_ref())?;
    let registry = SecretRegistry::resolve(&config);

    for secret in registry.all() {
        if secret.value.is_some() {
            eprintln!("  ok  {}", secret.name);
        } else {
            eprintln!("  MISSING  {} — {}", secret.name, secret.error.as_deref().unwrap_or("unknown"));
        }
    }

    if registry.all_resolved() {
        eprintln!("all {} secrets available", registry.names().len());
        Ok(())
    } else {
        let missing = registry.missing();
        eprintln!("{} of {} secrets missing", missing.len(), registry.names().len());
        std::process::exit(1);
    }
}

fn cmd_run(root: &Utf8PathBuf, run_args: &RunArgs) -> Result<()> {
    let config = BelmontConfig::load(root.as_ref())?;
    let registry = SecretRegistry::resolve(&config);

    if !registry.all_resolved() {
        let missing = registry.missing();
        eprintln!(
            "belmont: {} secret(s) missing, run `belmont check` for details",
            missing.len()
        );
        std::process::exit(1);
    }

    let pairs = registry.resolved_pairs();
    let mut scrubber = Scrubber::new(pairs.clone());
    let env_map = registry.env_map();

    let exit_code = runner::run_command(&run_args.command, env_map, &mut scrubber)?;
    std::process::exit(exit_code);
}
