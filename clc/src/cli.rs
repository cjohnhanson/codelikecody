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
    Init {
        /// Keep clc files invisible to git via .git/info/exclude.
        #[arg(long)]
        untracked: bool,
    },
    /// Process a hook event from stdin (called by agent hooks).
    Hook,
    /// Show current clc state (branch, phase, etc.).
    Status {
        #[command(subcommand)]
        action: Option<StatusAction>,
    },
    /// Pick up a tisket: create worktree, set status, initialize phase.
    Pickup {
        /// The tisket issue ID to pick up.
        id: String,
    },
    /// Finalize work: advance phase to done, close tisket.
    Done,
    /// Print the assembled prime text (agent orientation + directives).
    Prime,
    /// View and manage clc configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Run tisket commands.
    Tisket {
        #[command(subcommand)]
        command: ::tisket::cli::Command,
    },
    /// Run missouri commands.
    Missouri {
        #[command(subcommand)]
        command: ::missouri::cli::Command,
    },
}

#[derive(Subcommand)]
pub enum StatusAction {
    /// Set the current workflow phase.
    Set {
        /// The phase to transition to.
        phase: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print the effective configuration.
    Show,
}
