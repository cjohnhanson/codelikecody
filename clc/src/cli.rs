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
        /// Overwrite existing hooks in settings.local.json.
        #[arg(long)]
        force: bool,
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
    /// Create or switch to the admin worktree for non-feature work.
    Admin,
    /// Print the main repository root path (for navigating back to trunk).
    Home,
    /// Merge a completed feature branch into trunk.
    Merge {
        /// The branch (tisket ID) to merge.
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
    /// Run the coordinator: dispatch pickable tiskets to worker agents.
    Coordinate {
        /// Maximum budget per worker in USD.
        #[arg(long, default_value = "5.0")]
        budget: f64,
        /// Model to use for workers.
        #[arg(long, default_value = "opus")]
        model: String,
        /// Only process this specific tisket (instead of all pickable ones).
        #[arg(long)]
        tisket: Option<String>,
    },
    /// Dispatch a worker: pickup tisket + spawn detached claude process.
    Dispatch {
        /// The tisket issue ID to dispatch.
        id: String,
        /// Model to use for the worker.
        #[arg(long, default_value = "sonnet")]
        model: String,
        /// Maximum budget in USD.
        #[arg(long, default_value = "5.0")]
        budget: f64,
    },
    /// List active workers and their status.
    Workers,
    /// Interact with a specific worker.
    Worker {
        /// The worker ID (tisket ID).
        id: String,
        #[command(subcommand)]
        action: WorkerAction,
    },
    /// Land a completed worker: stop, verify, merge, cleanup.
    Land {
        /// The worker ID (tisket ID) to land.
        id: String,
    },
    /// Run missouri commands.
    Missouri {
        #[command(subcommand)]
        command: ::missouri::cli::Command,
    },
}

#[derive(Subcommand)]
pub enum WorkerAction {
    /// Show activity since last check (cursor-based).
    Check,
    /// Show parsed output log.
    Log {
        /// Number of lines to show.
        #[arg(long, default_value = "50")]
        lines: usize,
    },
    /// Send a follow-up message to the worker.
    Send {
        /// The message to send.
        message: String,
    },
    /// Stop the worker process (leave worktree intact).
    Stop,
    /// Show raw NDJSON output.
    Raw {
        /// Number of lines to show (from end). 0 = all.
        #[arg(long, default_value = "10")]
        lines: usize,
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
