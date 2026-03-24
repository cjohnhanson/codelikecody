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
    /// List and retrieve agent skills from configured sources.
    Almanac {
        #[command(subcommand)]
        command: ::almanac::cli::Command,
    },
    /// Browse bundled documentation.
    Docs {
        /// Topic slug to display, or "search" to search.
        topic: Option<String>,

        /// Search query (when topic is "search").
        query: Option<String>,
    },
    /// Run tisket commands.
    Tisket {
        #[command(subcommand)]
        command: ::tisket::cli::Command,
    },
    /// Start the supervisor: spawn coordinators, monitor health, surface escalations.
    Up,
    /// Run a coordinator process (started by the supervisor, not by humans).
    #[command(name = "coordinator-run")]
    CoordinatorRun {
        /// Coordinator ID.
        #[arg(long)]
        id: String,
        /// Maximum concurrent workers for this coordinator.
        #[arg(long, default_value = "3")]
        max_workers: usize,
        /// Model to use for workers.
        #[arg(long, default_value = "opus")]
        model: String,
        /// Only tiskets in this project.
        #[arg(long)]
        project: Option<String>,
        /// Only tiskets with this label.
        #[arg(long)]
        label: Option<String>,
        /// Skip tiskets with this label.
        #[arg(long)]
        exclude_label: Option<String>,
        /// Permission pattern to auto-grant (repeatable).
        #[arg(long)]
        auto_grant: Vec<String>,
        /// Permission pattern to always escalate (repeatable).
        #[arg(long)]
        always_escalate: Vec<String>,
        /// Poll interval in seconds.
        #[arg(long, default_value = "10")]
        poll_interval: u64,
        /// Workspace type: worktree or docker.
        #[arg(long, default_value = "worktree")]
        workspace: String,
        /// Docker image to use (when workspace=docker).
        #[arg(long)]
        docker_image: Option<String>,
    },
    /// Run the coordinator: dispatch pickable tiskets to worker agents.
    Coordinate {
        /// Model to use for workers.
        #[arg(long, default_value = "opus")]
        model: String,
        /// Only process this specific tisket (instead of all pickable ones).
        #[arg(long)]
        tisket: Option<String>,
        /// Only tiskets with this label.
        #[arg(long)]
        label: Option<String>,
        /// Skip tiskets with this label.
        #[arg(long)]
        exclude_label: Option<String>,
        /// Only tiskets in this project.
        #[arg(long)]
        project: Option<String>,
        /// Only tiskets in the dependency chain rooted at this id.
        #[arg(long)]
        depends_on: Option<String>,
        /// Filter by comma-separated selectors (e.g. "label:feature,project:v0.1.0").
        #[arg(long)]
        filter: Option<String>,
        /// List pickable tiskets and exit without spawning a coordinator.
        #[arg(long)]
        dry_run: bool,
        /// Unique identity for this coordinator (e.g., coord-infra, coord-ui).
        #[arg(long)]
        id: Option<String>,
        /// Permission pattern the coordinator can auto-grant to workers (repeatable).
        #[arg(long)]
        auto_grant: Vec<String>,
        /// Escalate all permission requests to the user (conservative mode).
        #[arg(long)]
        escalate_all: bool,
        /// Path to an external permission policy YAML file.
        #[arg(long)]
        grant_config: Option<String>,
    },
    /// Dispatch a worker: pickup tisket + spawn detached claude process.
    Dispatch {
        /// The tisket issue ID to dispatch.
        id: String,
        /// Model to use for the worker.
        #[arg(long, default_value = "opus")]
        model: String,
        /// Coordinator ID claiming this tisket.
        #[arg(long)]
        coordinator_id: Option<String>,
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
    /// Run zettel commands.
    Zettel {
        #[command(subcommand)]
        command: ::zettel::cli::Command,
    },
    /// Run belmont commands.
    Belmont {
        #[command(subcommand)]
        command: ::belmont::cli::Command,
    },
    /// Manage worker permissions (request, grant, list).
    Permissions {
        #[command(subcommand)]
        action: PermissionsAction,
    },
    /// Poll the inbox for new items (.clc/inbox/).
    Inbox {
        #[command(subcommand)]
        action: InboxAction,
    },
    /// Write items to the outbox (one file per item in .clc/outbox/).
    Outbox {
        #[command(subcommand)]
        action: OutboxAction,
    },
    /// Manage ephemeral integration branches (create, merge workers, land).
    Integrate {
        #[command(subcommand)]
        action: IntegrateAction,
    },
    /// Sleep, then print a message. For background reminders that fire after a delay.
    Remind {
        /// Seconds to sleep before printing the message.
        seconds: u64,
        /// The message to print when the timer fires.
        message: String,
        /// Number of times to repeat. Appends a re-run instruction with --repeat N-1.
        #[arg(long, default_value = "0")]
        repeat: u32,
    },
    /// Workspace setup commands (run inside workspaces, not by humans).
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    /// List running coordinators with status.
    Coordinators {
        /// Show all coordinators including dead ones.
        #[arg(long)]
        all: bool,
    },
    /// Interact with a specific coordinator.
    Coordinator {
        /// The coordinator ID.
        id: String,
        #[command(subcommand)]
        action: CoordinatorAction,
    },
}

#[derive(Subcommand)]
pub enum InboxAction {
    /// Poll the inbox directory, printing items as JSON and moving them to .processed/.
    Poll,
}

#[derive(Subcommand)]
pub enum OutboxAction {
    /// Write an item to the outbox, reading content from stdin.
    Write {
        /// Filename for the item (e.g. "summary.md", "result.json").
        name: String,
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
    /// Deny a permission escalation (called by admin or user).
    Deny {
        /// The worker ID (tisket ID) this denial is about.
        worker_id: String,
        /// Reason for denying the permission request.
        reason: String,
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
pub enum CoordinatorAction {
    /// Show activity since last check (cursor-based).
    Check,
    /// Show parsed output log.
    Log {
        /// Number of lines to show.
        #[arg(long, default_value = "50")]
        lines: usize,
    },
    /// Send a follow-up message to the coordinator.
    Send {
        /// The message to send.
        message: String,
    },
    /// Stop the coordinator process.
    Stop,
    /// Squash-merge the coordinator's integration branch into main.
    Land,
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

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// Initialize a workspace: create project dir, .clc/worker/ with stdio pipes.
    Init,
    /// Write stdin to a file. Used by the supervisor to deploy files.
    WriteFile {
        /// Path to write to.
        path: String,
    },
    /// Export the current branch as a pack (JSON envelope to stdout).
    Export {
        /// Branch to export.
        #[arg(long)]
        branch: String,
    },
    /// Start the agent process with stdio wired to pipes/files.
    Start {
        /// Agent model.
        #[arg(long, default_value = "opus")]
        model: String,
        /// Branch / tisket ID.
        #[arg(long)]
        branch: String,
        /// CLC API URL for coordination.
        #[arg(long)]
        api_url: Option<String>,
        /// OAuth token for the agent.
        #[arg(long)]
        oauth_token: Option<String>,
    },
    /// Receive a repo bundle and extract it, checking out the specified branch.
    Receive {
        /// Path to the bundle file (omit if using --stdin).
        bundle: Option<String>,
        /// Branch to checkout.
        #[arg(long)]
        branch: String,
        /// Read bundle from stdin instead of a file.
        #[arg(long)]
        stdin: bool,
        /// Target directory (default: current directory).
        #[arg(long)]
        dir: Option<String>,
    },
}
