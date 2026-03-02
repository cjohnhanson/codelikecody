//! Dispatch: pickup a tisket and spawn a detached claude worker.
//!
//! The worker runs as a background process with:
//! - Named pipe (FIFO) for stdin (allows follow-up messages)
//! - File for stdout (NDJSON)
//! - File for stderr
//! - PID file for process management
//!
//! All worker state lives in `.clc/worker/` inside the worktree.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use camino::Utf8Path;
use nix::sys::stat::Mode;
use nix::unistd::{self, Pid};

use crate::error::Error;
use crate::git;
use crate::permissions;
use crate::pickup;

/// Worker state directory name inside `.clc/`.
const WORKER_DIR: &str = "worker";

pub fn dispatch(
    project_dir: &Path,
    id: &str,
    main_branch: &str,
    model: &str,
    extra_allow: &[String],
) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "must be on the main branch to dispatch (currently on '{}')",
            git_state.branch
        )));
    }

    let worktree_dir = project_dir.join(".worktrees").join(id);

    // Check if a worker is already running for this tisket.
    if is_worker_alive(&worktree_dir) {
        return Err(Error::NonBlocking(format!(
            "worker already running for '{id}'"
        )));
    }

    // Pickup creates worktree, sets tisket status, inits clc.
    // If the worktree already exists (previous dispatch), pickup will fail,
    // which is the correct behavior — re-dispatch after `land` or manual cleanup.
    pickup::pickup(project_dir, id, main_branch)?;

    // Seed baseline permissions so the worker can function without
    // --dangerously-skip-permissions.
    permissions::seed_baseline(&worktree_dir, extra_allow)?;

    // Build prompts.
    let initial_prompt = build_worker_prompt(project_dir, id)?;
    let system_prompt = build_system_prompt(id);

    // Spawn the worker.
    let worker_dir = worktree_dir.join(".clc").join(WORKER_DIR);
    let pid = spawn_worker_process(
        &worktree_dir,
        &worker_dir,
        model,
        &system_prompt,
        &initial_prompt,
        &[],
    )?;

    eprintln!("dispatched worker for '{id}' (pid {pid})");

    Ok(())
}

/// Spawn a claude --print process with pipe infrastructure.
///
/// Creates the worker state directory with stdin.pipe, stdout.jsonl,
/// stderr.log, and pid file. Sends the initial prompt via the pipe.
///
/// Returns the child PID.
pub fn spawn_worker_process(
    working_dir: &Path,
    worker_dir: &Path,
    model: &str,
    system_prompt: &str,
    initial_prompt: &str,
    extra_args: &[&str],
) -> Result<u32, Error> {
    fs::create_dir_all(worker_dir)?;

    let pid_path = worker_dir.join("pid");
    let stdout_path = worker_dir.join("stdout.jsonl");
    let stderr_path = worker_dir.join("stderr.log");
    let stdin_pipe_path = worker_dir.join("stdin.pipe");

    // Create the named pipe for stdin.
    create_named_pipe(&stdin_pipe_path)?;

    // Open stdout and stderr files.
    let stdout_file = fs::File::create(&stdout_path)?;
    let stderr_file = fs::File::create(&stderr_path)?;

    // Open the named pipe for reading (will be the child's stdin).
    // This must be opened with O_RDWR to prevent blocking on open and to keep
    // the read end alive even when no writer is connected.
    let stdin_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&stdin_pipe_path)?;

    // Build the claude command.
    let mut cmd = Command::new("claude");
    cmd.current_dir(working_dir);
    cmd.arg("--print");
    cmd.arg("--verbose");
    cmd.arg("--input-format").arg("stream-json");
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--model").arg(model);
    cmd.arg("--append-system-prompt").arg(system_prompt);

    for arg in extra_args {
        cmd.arg(arg);
    }

    cmd.stdin(Stdio::from(stdin_file));
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));

    // Clear CLAUDECODE so the child doesn't think it's nested.
    cmd.env_remove("CLAUDECODE");

    // Spawn the worker.
    let child = cmd
        .spawn()
        .map_err(|e| Error::NonBlocking(format!("failed to spawn claude worker: {e}")))?;

    let pid = child.id();

    // Write PID file.
    fs::write(&pid_path, pid.to_string())?;

    // Send the initial prompt via the named pipe.
    send_prompt(&stdin_pipe_path, initial_prompt)?;

    Ok(pid)
}

pub fn is_worker_alive(worktree_dir: &Path) -> bool {
    let pid_path = worktree_dir.join(".clc").join(WORKER_DIR).join("pid");
    let Ok(contents) = fs::read_to_string(&pid_path) else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<i32>() else {
        return false;
    };

    // Signal 0 checks if the process exists without sending a signal.
    nix::sys::signal::kill(Pid::from_raw(pid), None).is_ok()
}

fn create_named_pipe(path: &Path) -> Result<(), Error> {
    // Remove existing pipe if any.
    if path.exists() {
        fs::remove_file(path)?;
    }

    unistd::mkfifo(path, Mode::S_IRUSR | Mode::S_IWUSR)
        .map_err(|e| Error::NonBlocking(format!("mkfifo: {e}")))?;

    Ok(())
}

pub fn send_prompt(pipe_path: &Path, prompt: &str) -> Result<(), Error> {
    let input = claude_code::protocol::InputMessage::user(prompt);
    let json = serde_json::to_string(&input)?;

    let mut file = fs::OpenOptions::new().write(true).open(pipe_path)?;
    writeln!(file, "{json}")?;
    file.flush()?;

    Ok(())
}

fn build_worker_prompt(project_dir: &Path, tisket_id: &str) -> Result<String, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issue = repo
        .find_issue(tisket_id)
        .map_err(|e| Error::NonBlocking(format!("tisket '{tisket_id}': {e}")))?;

    Ok(format!(
        "You are working on tisket '{tisket_id}': {}\n\n\
         {}\n\n\
         Follow the clc workflow: write tests, implement, get green, run `clc done`.\n\
         The hooks will guide you through each phase.",
        issue.frontmatter.title, issue.body,
    ))
}

fn build_system_prompt(tisket_id: &str) -> String {
    format!(
        "You are an autonomous worker agent managed by clc. \
         Your task is defined by tisket '{tisket_id}'. \
         Follow the phase system: tests-unwritten -> tests-written -> red -> implementing -> green -> done. \
         When all tests pass and work is complete, run `clc done` to finalize. \
         Do not stop before reaching the 'done' phase.\n\n\
         Your permissions are limited. You have access to file operations, search, \
         web lookup, clc/tisket/missouri/cargo commands, and basic shell commands. \
         If a tool call is denied, do not retry it. Run \
         `clc permissions request \"<what you need and why>\"` and stop working. \
         The coordinator will review your request and either grant the permission \
         or send you new instructions. Wait to be resumed."
    )
}
