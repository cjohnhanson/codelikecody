use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "clc",
    about = "codelikecody — workflow enforcement for coding agents"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize clc in the current project directory.
    Init,
    /// Process a hook event from stdin (called by agent hooks).
    Hook,
    /// View and manage clc configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print the effective configuration.
    Show,
}
