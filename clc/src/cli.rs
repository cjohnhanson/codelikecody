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
    },
    /// List active workers and their status.
    Workers {
        /// Show all workers including dead ones.
        #[arg(long)]
        all: bool,
        /// Remove worker state files for dead workers.
        #[arg(long)]
        prune: bool,
        /// Show stranded workers: worktrees with no alive process and a phase set.
        #[arg(long)]
        stranded: bool,
    },
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
    /// Manage worker permissions (request, grant, list).
    Permissions {
        #[command(subcommand)]
        action: PermissionsAction,
    },
    /// Manage ephemeral integration branches (create, merge workers, land).
    Integrate {
        #[command(subcommand)]
        action: IntegrateAction,
    },
}

#[derive(Subcommand)]
pub enum IntegrateAction {
    /// Create a new integration branch at the current main HEAD.
    Create {
        /// Name for the integration branch (creates integrate/<name>).
        name: String,
    },
    /// Merge a worker branch into the current integration branch.
    Merge {
        /// The branch name to merge.
        branch: String,
    },
    /// Squash-merge the integration branch onto main and clean up.
    Land,
}

#[derive(Subcommand)]
pub enum PermissionsAction {
    /// Request a permission (called from within a worker worktree).
    Request {
        /// Description of what permission is needed and why.
        description: String,
    },
    /// Grant a permission to a worker (called by the coordinator).
    Grant {
        /// The worker ID (tisket ID).
        worker_id: String,
        /// The permission to grant (e.g. "Bash(npm install:*)").
        permission: String,
    },
    /// List all pending permission requests across workers.
    List,
    /// Escalate a permission decision to the user (called by the coordinator).
    Escalate {
        /// The worker ID (tisket ID) this escalation is about.
        worker_id: String,
        /// Description of what the worker needs and why it requires user review.
        description: String,
    },
    /// View pending escalations from the coordinator.
    Inbox,
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
    /// Resume a stopped worker (re-attach to existing session).
    Resume,
    /// Supervise a worker: auto-resume if it stops before reaching done.
    Supervise {
        /// Maximum number of auto-resumes before giving up.
        #[arg(long, default_value = "3")]
        max_resumes: u32,
    },
    /// Show raw NDJSON output.
    Raw {
        /// Number of lines to show (from end). 0 = all.
        #[arg(long, default_value = "10")]
        lines: usize,
    },
    /// Recover a stranded worker: finalize work without re-dispatching.
    Recover,
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
