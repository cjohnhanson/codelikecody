//! Worker interaction: list, check, log, send, stop, resume, raw, land.
//!
//! Worker state lives in `.clc/worker/` inside each worktree.
//! Coordinator state (cursors) lives in `.clc/workers/<id>/` on trunk.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use clc_sdk::agent::Agent;

use claude_code::protocol::{ContentBlock, OutputMessage};
use nix::sys::signal::{self, Signal};
use nix::sys::stat::Mode;
use nix::unistd::{self, Pid};

use crate::coordination::Coordination;
use crate::error::Error;
use crate::merge;

/// Worker state directory name inside the worktree.
const WORKER_DIR: &str = "worker";

struct WorkerInfo {
    id: String,
    pid: Option<i32>,
    alive: bool,
    line_count: usize,
    last_activity: String,
}

/// List workers across worktrees. By default only shows live workers; pass `all=true` to include dead ones.
pub fn list_workers(project_dir: &Path, all: bool) -> Result<(), Error> {
    let mut workers = collect_workers(project_dir)?;

    // Enrich with coordination DB status if available.
    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            for w in &mut workers {
                if let Ok(status) = coord.get_status(&w.id) {
                    w.alive = status == clc_sdk::coordination::AgentStatus::Running;
                }
            }
        }
    }

    let visible: Vec<&WorkerInfo> = workers.iter().filter(|w| all || w.alive).collect();

    if visible.is_empty() {
        eprintln!("no workers");
        return Ok(());
    }

    for w in &visible {
        let status = if w.alive { "working" } else { "dead" };
        let pid_str = w.pid.map_or_else(|| "?".to_string(), |p| p.to_string());
        println!(
            "{}\t{}\tpid={}\tlines={}\t{}",
            w.id, status, pid_str, w.line_count, w.last_activity
        );
    }

    Ok(())
}

/// Remove worker state files for dead workers.
/// Deletes `.clc/worker/` from each worktree where the worker PID is dead,
/// and removes coordinator cursor dirs (`.clc/workers/<id>/`) from the project root.
pub fn prune_workers(project_dir: &Path) -> Result<(), Error> {
    let workers = collect_workers(project_dir)?;

    let dead: Vec<&WorkerInfo> = workers.iter().filter(|w| !w.alive).collect();

    if dead.is_empty() {
        eprintln!("no dead workers to prune");
        return Ok(());
    }

    for w in &dead {
        let wdir = worker_dir_for(project_dir, &w.id);
        if wdir.is_dir() {
            fs::remove_dir_all(&wdir)?;
            eprintln!("pruned worker state for '{}'", w.id);
        }
        // Remove coordinator cursor dir from project root.
        let cursor_dir = project_dir.join(".clc").join("workers").join(&w.id);
        if cursor_dir.is_dir() {
            fs::remove_dir_all(&cursor_dir)?;
        }
    }

    Ok(())
}

/// Collect `WorkerInfo` for all worktrees that have a `.clc/worker/` directory,
/// plus the coordinator if it has worker state at the project root.
fn collect_workers(project_dir: &Path) -> Result<Vec<WorkerInfo>, Error> {
    let mut workers = Vec::new();

    // Check for coordinator worker state on trunk.
    let coord_worker_dir = worker_dir_for(project_dir, COORDINATOR_ID);
    if coord_worker_dir.is_dir() {
        let pid = read_pid(&coord_worker_dir);
        let alive = pid.is_some_and(is_process_alive);
        let line_count = count_lines(&coord_worker_dir.join("stdout.jsonl"));
        let last_activity = last_activity_summary(&coord_worker_dir.join("stdout.jsonl"));

        workers.push(WorkerInfo {
            id: COORDINATOR_ID.to_string(),
            pid,
            alive,
            line_count,
            last_activity,
        });
    }

    // Check worktrees for regular workers.
    let worktrees_dir = project_dir.join(".worktrees");
    if worktrees_dir.is_dir() {
        let entries = fs::read_dir(&worktrees_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let id = entry.file_name().to_str().unwrap_or("unknown").to_string();

            let wdir = path.join(".clc").join(WORKER_DIR);
            if !wdir.is_dir() {
                continue;
            }

            let pid = read_pid(&wdir);
            let alive = pid.is_some_and(is_process_alive);
            let line_count = count_lines(&wdir.join("stdout.jsonl"));
            let last_activity = last_activity_summary(&wdir.join("stdout.jsonl"));

            workers.push(WorkerInfo {
                id,
                pid,
                alive,
                line_count,
                last_activity,
            });
        }
    }

    Ok(workers)
}

/// Show activity since last check (cursor-based).
pub fn check(project_dir: &Path, id: &str) -> Result<(), Error> {
    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            if let Ok((msgs, _)) = coord.recv(id, &clc_sdk::coordination::Cursor::default()) {
                for msg in &msgs {
                    match &msg.kind {
                        clc_sdk::coordination::MessageKind::StatusUpdate { phase, detail } => {
                            eprintln!("[status] {phase}: {detail}");
                        }
                        clc_sdk::coordination::MessageKind::Output(text) => {
                            println!("{text}");
                        }
                        clc_sdk::coordination::MessageKind::PermissionRequest { tool_name, reason } => {
                            eprintln!("[permission-request] {tool_name}: {reason}");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // If CLC_API_URL is set, read output from the supervisor API.
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let cursor_path = cursor_path(project_dir, id);
        let cursor = read_cursor(&cursor_path);

        let url = format!("{api_url}/agents/{id}/output?after={cursor}");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

        let body: serde_json::Value = rt.block_on(async {
            crate::coordination_client::build_api_client()
                .map_err(|e| Error::NonBlocking(format!("http client: {e}")))?
                .get(&url)
                .send()
                .await
                .map_err(|e| Error::NonBlocking(format!("http: {e}")))?
                .json()
                .await
                .map_err(|e| Error::NonBlocking(format!("json: {e}")))
        })?;

        let lines = body["lines"].as_array();
        let new_cursor = body["cursor"].as_u64().unwrap_or(cursor as u64) as usize;

        if let Some(lines) = lines {
            if lines.is_empty() {
                eprintln!("no new activity for '{id}'");
            } else {
                for line in lines {
                    if let Some(s) = line.as_str() {
                        print_parsed_line(s);
                    }
                }
            }
        }

        write_cursor(&cursor_path, new_cursor)?;
        return Ok(());
    }

    // Fall back to local filesystem.
    let stdout_path = worker_stdout_path(project_dir, id);
    if !stdout_path.exists() {
        return Err(Error::NonBlocking(format!("no worker output for '{id}'")));
    }

    // Read cursor.
    let cursor_path = cursor_path(project_dir, id);
    let cursor = read_cursor(&cursor_path);

    // Read lines from cursor.
    let lines = read_stdout_lines(&stdout_path)?;
    let new_lines = &lines[cursor.min(lines.len())..];

    if new_lines.is_empty() {
        eprintln!("no new activity for '{id}'");
    } else {
        for line in new_lines {
            print_parsed_line(line);
        }
    }

    // Update cursor.
    write_cursor(&cursor_path, lines.len())?;

    // Surface any pending permission request.
    if let Some(description) = crate::permissions::pending_request(project_dir, id) {
        eprintln!("[PERMISSION REQUEST PENDING] {description}");
        eprintln!("  Grant with: clc permissions grant {id} \"<permission>\"");
    }

    Ok(())
}

/// Show parsed output log (last N entries).
pub fn log(project_dir: &Path, id: &str, max_lines: usize) -> Result<(), Error> {
    let stdout_path = worker_stdout_path(project_dir, id);
    if !stdout_path.exists() {
        return Err(Error::NonBlocking(format!("no worker output for '{id}'")));
    }

    let lines = read_stdout_lines(&stdout_path)?;
    let start = lines.len().saturating_sub(max_lines);

    for line in &lines[start..] {
        print_parsed_line(line);
    }

    Ok(())
}

/// Send a follow-up message to the worker via the named pipe or API.
pub fn send(project_dir: &Path, id: &str, message: &str) -> Result<(), Error> {
    // If CLC_API_URL is set, send through the supervisor API.
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let url = format!("{api_url}/agents/{id}/stdin");
        let body = serde_json::json!({ "message": message });

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

        let status = rt.block_on(async {
            crate::coordination_client::build_api_client()
                .map_err(|e| Error::NonBlocking(format!("http client: {e}")))?
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| Error::NonBlocking(format!("http: {e}")))
                .map(|r| r.status())
        })?;

        if !status.is_success() {
            return Err(Error::NonBlocking(format!(
                "failed to send to worker '{id}': HTTP {status}"
            )));
        }

        eprintln!("sent message to '{id}' via API");
        return Ok(());
    }

    // Fall back to local named pipe.
    let wdir = worker_dir_for(project_dir, id);
    let pipe_path = wdir.join("stdin.pipe");

    // Check the worker is alive.
    let pid = read_pid(&wdir)
        .ok_or_else(|| Error::NonBlocking(format!("no PID file for worker '{id}'")))?;
    if !is_process_alive(pid) {
        return Err(Error::NonBlocking(format!(
            "worker '{id}' is not running (pid {pid})"
        )));
    }

    if !pipe_path.exists() {
        return Err(Error::NonBlocking(format!(
            "no stdin pipe for worker '{id}'"
        )));
    }

    let input = claude_code::protocol::InputMessage::user(message);
    let json = serde_json::to_string(&input)?;

    let mut file = fs::OpenOptions::new().write(true).open(&pipe_path)?;
    writeln!(file, "{json}")?;
    file.flush()?;

    eprintln!("sent message to worker '{id}'");
    Ok(())
}

/// Stop the worker process. Leave worktree intact.
pub fn stop(project_dir: &Path, id: &str) -> Result<(), Error> {
    let wdir = worker_dir_for(project_dir, id);

    let pid = read_pid(&wdir)
        .ok_or_else(|| Error::NonBlocking(format!("no PID file for worker '{id}'")))?;

    if is_process_alive(pid) {
        let nix_pid = Pid::from_raw(pid);

        // Send SIGTERM.
        let _ = signal::kill(nix_pid, Signal::SIGTERM);

        // Wait briefly for process to exit.
        for _ in 0..20 {
            if !is_process_alive(pid) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Force kill if still alive.
        if is_process_alive(pid) {
            let _ = signal::kill(nix_pid, Signal::SIGKILL);
        }

        eprintln!("stopped worker '{id}' (pid {pid})");
    } else {
        eprintln!("worker '{id}' already dead (pid {pid})");
    }

    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Stopped);
        }
    }

    Ok(())
}

/// Resume a stopped worker by re-attaching to its existing session.
pub fn resume(project_dir: &Path, id: &str) -> Result<(), Error> {
    let work_dir = working_dir_for(project_dir, id);
    let wdir = worker_dir_for(project_dir, id);

    if !work_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no working directory for '{id}'"
        )));
    }

    // Must not already be running.
    if let Some(pid) = read_pid(&wdir)
        && is_process_alive(pid)
    {
        return Err(Error::NonBlocking(format!(
            "worker '{id}' is already running (pid {pid})"
        )));
    }

    // Extract session ID from stdout.
    let stdout_path = wdir.join("stdout.jsonl");
    let session_id = extract_session_id(&stdout_path)?;

    let pid_path = wdir.join("pid");
    let stderr_path = wdir.join("stderr.log");
    let stdin_pipe_path = wdir.join("stdin.pipe");

    // Recreate the named pipe.
    if stdin_pipe_path.exists() {
        fs::remove_file(&stdin_pipe_path)?;
    }
    unistd::mkfifo(&stdin_pipe_path, Mode::S_IRUSR | Mode::S_IWUSR)
        .map_err(|e| Error::NonBlocking(format!("mkfifo: {e}")))?;

    // Open stdout in append mode (preserve previous session output).
    let stdout_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_path)?;
    let stderr_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)?;

    // Open pipe with O_RDWR to avoid blocking.
    let stdin_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&stdin_pipe_path)?;

    // Build the resume command via the Agent trait.
    let agent = clc_sdk::agent::ClaudeCodeAgent::new();
    let mut cmd = agent
        .build_resume_command(&session_id, &work_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to build resume command: {e}")))?;

    cmd.stdin(Stdio::from(stdin_file));
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));

    let child = cmd
        .spawn()
        .map_err(|e| Error::NonBlocking(format!("failed to spawn agent: {e}")))?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())?;

    // Send a continuation prompt.
    let input = claude_code::protocol::InputMessage::user(
        "Continue where you left off. Run `clc status` to check current state and proceed through the phases.",
    );
    let json = serde_json::to_string(&input)?;
    let mut pipe = fs::OpenOptions::new().write(true).open(&stdin_pipe_path)?;
    writeln!(pipe, "{json}")?;
    pipe.flush()?;

    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Running);
            let _ = coord.set_pid(id, Some(pid.cast_signed()));
        }
    }

    eprintln!("resumed worker '{id}' (pid {pid}, session {session_id})");
    Ok(())
}

/// Supervise a worker: poll until it reaches done, auto-resuming if it stops early.
pub fn supervise(project_dir: &Path, id: &str, max_resumes: u32) -> Result<(), Error> {
    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            if let Ok(status) = coord.get_status(id) {
                if status == clc_sdk::coordination::AgentStatus::Completed {
                    eprintln!("worker '{id}' already completed (coordination DB)");
                    return Ok(());
                }
            }
        }
    }

    let work_dir = working_dir_for(project_dir, id);
    let wdir = worker_dir_for(project_dir, id);

    if !work_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no working directory for '{id}'"
        )));
    }

    let mut resumes = 0u32;

    loop {
        // Wait for the worker process to exit.
        wait_for_exit(&wdir);

        // Check phase.
        let phase = crate::phase::load(&work_dir).unwrap_or(None);

        if phase == Some(crate::phase::Phase::Done) {
            eprintln!("worker '{id}' reached done phase");
            return Ok(());
        }

        // Block on pending permission requests before attempting auto-resume.
        if let Some(description) = crate::permissions::pending_request(project_dir, id) {
            return Err(Error::NonBlocking(format!(
                "worker '{id}' has a pending permission request: \"{description}\"\n\
                 Grant with: clc permissions grant {id} \"<permission>\"\n\
                 Then resume supervision with: clc worker {id} supervise"
            )));
        }

        let phase_str = phase.map_or_else(|| "none".to_string(), |p| p.to_string());

        if resumes >= max_resumes {
            return Err(Error::NonBlocking(format!(
                "worker '{id}' stopped at phase '{phase_str}' after {resumes} resumes — giving up"
            )));
        }

        resumes += 1;
        eprintln!(
            "worker '{id}' stopped at phase '{phase_str}' — resuming ({resumes}/{max_resumes})"
        );

        resume(project_dir, id)?;
    }
}

/// Poll until the worker process exits.
fn wait_for_exit(worker_dir: &Path) {
    loop {
        if let Some(pid) = read_pid(worker_dir) {
            if !is_process_alive(pid) {
                return;
            }
        } else {
            // No PID file — treat as exited.
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

/// Extract the session ID from a worker's stdout.jsonl.
fn extract_session_id(stdout_path: &Path) -> Result<String, Error> {
    let lines = read_stdout_lines(stdout_path)?;

    // Look for the system init message which has the session_id.
    for line in &lines {
        if let Ok(OutputMessage::System(sys)) = serde_json::from_str::<OutputMessage>(line)
            && sys.subtype == "init"
            && let Some(id) = sys.session_id
        {
            return Ok(id);
        }
    }

    Err(Error::NonBlocking(
        "no session ID found in worker output".into(),
    ))
}

/// Show raw NDJSON output.
pub fn raw(project_dir: &Path, id: &str, max_lines: usize) -> Result<(), Error> {
    let stdout_path = worker_stdout_path(project_dir, id);
    if !stdout_path.exists() {
        return Err(Error::NonBlocking(format!("no worker output for '{id}'")));
    }

    let lines = read_stdout_lines(&stdout_path)?;
    let start = if max_lines > 0 {
        lines.len().saturating_sub(max_lines)
    } else {
        0
    };

    for line in &lines[start..] {
        println!("{line}");
    }

    Ok(())
}

/// Land a worker: stop if alive, merge to trunk, cleanup worktree.
pub fn land(project_dir: &Path, id: &str, main_branch: &str, admin_branch: &str) -> Result<(), Error> {
    let wdir = worker_dir_for(project_dir, id);

    // Stop worker if still alive.
    if let Some(pid) = read_pid(&wdir)
        && is_process_alive(pid)
    {
        stop(project_dir, id)?;
    }

    // Merge the branch (also removes worktree and branch).
    merge::merge(project_dir, id, main_branch, admin_branch)?;

    // Remove coordinator cursor dir from project root.
    let cursor_dir = project_dir.join(".clc").join("workers").join(id);
    if cursor_dir.is_dir() {
        let _ = fs::remove_dir_all(&cursor_dir);
    }

    eprintln!("landed '{id}' — merged into trunk");

    Ok(())
}

/// List stranded workers: worktrees with no alive process and a phase set.
/// These are workers that did work but died before completing the phase ceremony.
pub fn list_stranded(project_dir: &Path) -> Result<(), Error> {
    let stranded = collect_stranded(project_dir)?;

    if stranded.is_empty() {
        eprintln!("no stranded workers");
        return Ok(());
    }

    for s in &stranded {
        println!("{}\t{}\t{}", s.id, s.phase, s.status);
    }

    Ok(())
}

struct StrandedInfo {
    id: String,
    phase: String,
    status: String,
}

/// Scan worktrees for stranded workers: worktree exists, no alive process, phase is set.
fn collect_stranded(project_dir: &Path) -> Result<Vec<StrandedInfo>, Error> {
    let worktrees_dir = project_dir.join(".worktrees");
    if !worktrees_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut stranded = Vec::new();

    for entry in fs::read_dir(&worktrees_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_str().unwrap_or("unknown").to_string();

        // Check if a worker process is alive for this worktree.
        let wdir = path.join(".clc").join(WORKER_DIR);
        if let Some(pid) = read_pid(&wdir)
            && is_process_alive(pid)
        {
            continue; // Worker is alive, not stranded.
        }

        // Load phase from the worktree.
        let Ok(Some(phase)) = crate::phase::load(&path) else {
            continue; // No phase set — not a managed worktree.
        };

        // Skip workers already at done — they just need landing, not recovery.
        if phase == crate::phase::Phase::Done {
            continue;
        }

        let status = if wdir.is_dir() {
            "dead (worker state exists)".to_string()
        } else {
            "dead (no worker state)".to_string()
        };

        stranded.push(StrandedInfo {
            id,
            phase: phase.to_string(),
            status,
        });
    }

    Ok(stranded)
}

/// Recover a stranded worker: run the done ceremony on its worktree without re-dispatching.
/// The worker must be dead and at the `green` phase.
pub fn recover(project_dir: &Path, id: &str, main_branch: &str, admin_branch: &str) -> Result<(), Error> {
    let work_dir = working_dir_for(project_dir, id);
    if !work_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no working directory for '{id}'"
        )));
    }

    // Must not be alive.
    let wdir = worker_dir_for(project_dir, id);
    if let Some(pid) = read_pid(&wdir)
        && is_process_alive(pid)
    {
        return Err(Error::NonBlocking(format!(
            "worker '{id}' is still alive (pid {pid}) — stop it first"
        )));
    }

    // Check phase.
    let phase = crate::phase::load(&work_dir)?
        .ok_or_else(|| Error::NonBlocking(format!("no phase set for worker '{id}'")))?;

    if phase == crate::phase::Phase::Done {
        return Err(Error::NonBlocking(format!(
            "worker '{id}' is already done — use `clc land {id}` to merge"
        )));
    }

    if phase != crate::phase::Phase::Green {
        return Err(Error::NonBlocking(format!(
            "worker '{id}' is at phase '{phase}', not 'green' — use `clc worker {id} resume` to continue the work"
        )));
    }

    // Advance through review phases to done (manual recovery bypasses coordinator review).
    crate::phase::set(&work_dir, "review-requested", 1)?;
    crate::phase::set(&work_dir, "in-review", 1)?;
    crate::phase::set(&work_dir, "reviewed", 1)?;
    crate::phase::set(&work_dir, "done", 1)?;

    // Run the done ceremony on the worktree.
    crate::done::done(&work_dir, main_branch, admin_branch)?;

    eprintln!("recovered worker '{id}' — phase advanced to done");

    Ok(())
}

// --- Helpers ---

fn worker_stdout_path(project_dir: &Path, id: &str) -> std::path::PathBuf {
    worker_dir_for(project_dir, id).join("stdout.jsonl")
}

fn cursor_path(project_dir: &Path, id: &str) -> std::path::PathBuf {
    project_dir
        .join(".clc")
        .join("workers")
        .join(id)
        .join("cursor")
}

fn read_cursor(path: &Path) -> usize {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_cursor(path: &Path, value: usize) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value.to_string())?;
    Ok(())
}

fn read_pid(worker_dir: &Path) -> Option<i32> {
    let pid_path = worker_dir.join("pid");
    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn is_process_alive(pid: i32) -> bool {
    // Signal 0 checks if the process exists without sending a signal.
    signal::kill(Pid::from_raw(pid), None).is_ok()
}

fn read_stdout_lines(path: &Path) -> Result<Vec<String>, Error> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect())
}

fn count_lines(path: &Path) -> usize {
    read_stdout_lines(path).map(|l| l.len()).unwrap_or(0)
}

fn last_activity_summary(stdout_path: &Path) -> String {
    let Ok(lines) = read_stdout_lines(stdout_path) else {
        return "none".to_string();
    };

    // Parse the last line to get a summary.
    lines.last().map_or_else(
        || "none".to_string(),
        |last| match serde_json::from_str::<OutputMessage>(last) {
            Ok(OutputMessage::System(_)) => "system".to_string(),
            Ok(OutputMessage::Assistant(a)) => {
                let tools: Vec<&str> = a
                    .message
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let ContentBlock::ToolUse { name, .. } = b {
                            Some(name.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                if tools.is_empty() {
                    "text".to_string()
                } else {
                    format!("tool:{}", tools.join(","))
                }
            }
            Ok(OutputMessage::User(_)) => "user".to_string(),
            Ok(OutputMessage::Result(r)) => {
                format!("result(${:.4})", r.cost_usd)
            }
            Err(_) => "unknown".to_string(),
        },
    )
}

fn print_parsed_line(line: &str) {
    match serde_json::from_str::<OutputMessage>(line) {
        Ok(OutputMessage::System(sys)) => {
            println!("[system] {}", sys.subtype);
        }
        Ok(OutputMessage::Assistant(a)) => {
            for block in &a.message.content {
                match block {
                    ContentBlock::Text { text } => {
                        println!("[text] {text}");
                    }
                    ContentBlock::ToolUse { name, .. } => {
                        println!("[tool] {name}");
                    }
                    ContentBlock::ToolResult { .. } => {
                        println!("[tool_result]");
                    }
                    ContentBlock::Thinking { .. } => {
                        println!("[thinking]");
                    }
                }
            }
        }
        Ok(OutputMessage::User(_)) => {
            println!("[user]");
        }
        Ok(OutputMessage::Result(r)) => {
            println!(
                "[result] cost=${:.4}, duration={:.1}s, error={}",
                r.cost_usd, r.duration_secs, r.is_error
            );
        }
        Err(_) => {
            println!("[raw] {line}");
        }
    }
}

/// Coordinator worker ID.
pub const COORDINATOR_ID: &str = "coordinator";

/// Resolve the worker state directory for a given ID.
///
/// Regular workers: `.worktrees/{id}/.clc/worker/`
/// Coordinator: `.clc/worker/` (on trunk root)
pub fn worker_dir_for(project_dir: &Path, id: &str) -> std::path::PathBuf {
    if id == COORDINATOR_ID {
        project_dir.join(".clc").join(WORKER_DIR)
    } else {
        project_dir
            .join(".worktrees")
            .join(id)
            .join(".clc")
            .join(WORKER_DIR)
    }
}

/// Resolve the working directory for a given worker ID.
///
/// Regular workers: `.worktrees/{id}/`
/// Coordinator: project root (trunk)
pub fn working_dir_for(project_dir: &Path, id: &str) -> std::path::PathBuf {
    if id == COORDINATOR_ID {
        project_dir.to_path_buf()
    } else {
        project_dir.join(".worktrees").join(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_dir() -> PathBuf {
        #[allow(deprecated)]
        let dir = tempfile::tempdir().unwrap().into_path();
        dir
    }

    fn create_worker_state(worker_dir: &Path, pid: i32) {
        fs::create_dir_all(worker_dir).unwrap();
        fs::write(worker_dir.join("pid"), pid.to_string()).unwrap();
        fs::write(worker_dir.join("stdout.jsonl"), "").unwrap();
        fs::write(worker_dir.join("stderr.log"), "").unwrap();
    }

    #[test]
    fn worker_dir_for_regular_worker() {
        let project = PathBuf::from("/tmp/project");
        let dir = worker_dir_for(&project, "my-tisket");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/project/.worktrees/my-tisket/.clc/worker")
        );
    }

    #[test]
    fn worker_dir_for_coordinator() {
        let project = PathBuf::from("/tmp/project");
        let dir = worker_dir_for(&project, COORDINATOR_ID);
        assert_eq!(dir, PathBuf::from("/tmp/project/.clc/worker"));
    }

    #[test]
    fn working_dir_for_regular_worker() {
        let project = PathBuf::from("/tmp/project");
        let dir = working_dir_for(&project, "my-tisket");
        assert_eq!(dir, PathBuf::from("/tmp/project/.worktrees/my-tisket"));
    }

    #[test]
    fn working_dir_for_coordinator() {
        let project = PathBuf::from("/tmp/project");
        let dir = working_dir_for(&project, COORDINATOR_ID);
        assert_eq!(dir, PathBuf::from("/tmp/project"));
    }

    #[test]
    fn collect_workers_finds_coordinator_on_trunk() {
        let project = make_test_dir();

        // Create coordinator worker state at project root.
        let coord_worker_dir = project.join(".clc").join("worker");
        create_worker_state(&coord_worker_dir, 99999);

        let workers = collect_workers(&project).unwrap();
        let coord = workers.iter().find(|w| w.id == COORDINATOR_ID);
        assert!(coord.is_some(), "coordinator should appear in worker list");
    }

    #[test]
    fn collect_workers_finds_both_coordinator_and_regular_workers() {
        let project = make_test_dir();

        // Create coordinator.
        let coord_worker_dir = project.join(".clc").join("worker");
        create_worker_state(&coord_worker_dir, 99999);

        // Create a regular worker.
        let worktree_worker_dir = project
            .join(".worktrees")
            .join("my-tisket")
            .join(".clc")
            .join("worker");
        create_worker_state(&worktree_worker_dir, 99998);

        let workers = collect_workers(&project).unwrap();
        assert_eq!(workers.len(), 2);

        let ids: Vec<&str> = workers.iter().map(|w| w.id.as_str()).collect();
        assert!(ids.contains(&COORDINATOR_ID));
        assert!(ids.contains(&"my-tisket"));
    }

    #[test]
    fn collect_workers_no_coordinator_without_state() {
        let project = make_test_dir();

        // No .clc/worker/ at project root.
        let workers = collect_workers(&project).unwrap();
        assert!(
            !workers.iter().any(|w| w.id == COORDINATOR_ID),
            "coordinator should not appear without .clc/worker/"
        );
    }
}
