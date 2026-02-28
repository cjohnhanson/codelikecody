//! Worker interaction: list, check, log, send, stop, raw, land.
//!
//! Worker state lives in `.clc/worker/` inside each worktree.
//! Coordinator state (cursors) lives in `.clc/workers/<id>/` on trunk.

use std::fs;
use std::io::Write;
use std::path::Path;

use claude_code::protocol::{ContentBlock, OutputMessage};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

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

/// List all workers across worktrees.
pub fn list_workers(project_dir: &Path) -> Result<(), Error> {
    let worktrees_dir = project_dir.join(".worktrees");
    if !worktrees_dir.is_dir() {
        eprintln!("no workers");
        return Ok(());
    }

    let mut workers = Vec::new();

    let entries = fs::read_dir(&worktrees_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_str().unwrap_or("unknown").to_string();

        let worker_dir = path.join(".clc").join(WORKER_DIR);
        if !worker_dir.is_dir() {
            continue;
        }

        let pid = read_pid(&worker_dir);
        let alive = pid.is_some_and(is_process_alive);
        let line_count = count_lines(&worker_dir.join("stdout.jsonl"));
        let last_activity = last_activity_summary(&worker_dir.join("stdout.jsonl"));

        workers.push(WorkerInfo {
            id,
            pid,
            alive,
            line_count,
            last_activity,
        });
    }

    if workers.is_empty() {
        eprintln!("no workers");
        return Ok(());
    }

    for w in &workers {
        let status = if w.alive { "working" } else { "dead" };
        let pid_str = w.pid.map_or_else(|| "?".to_string(), |p| p.to_string());
        println!(
            "{}\t{}\tpid={}\tlines={}\t{}",
            w.id, status, pid_str, w.line_count, w.last_activity
        );
    }

    Ok(())
}

/// Show activity since last check (cursor-based).
pub fn check(project_dir: &Path, id: &str) -> Result<(), Error> {
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

/// Send a follow-up message to the worker via the named pipe.
pub fn send(project_dir: &Path, id: &str, message: &str) -> Result<(), Error> {
    let worktree_dir = project_dir.join(".worktrees").join(id);
    let worker_dir = worktree_dir.join(".clc").join(WORKER_DIR);
    let pipe_path = worker_dir.join("stdin.pipe");

    // Check the worker is alive.
    let pid = read_pid(&worker_dir)
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
    let worktree_dir = project_dir.join(".worktrees").join(id);
    let worker_dir = worktree_dir.join(".clc").join(WORKER_DIR);

    let pid = read_pid(&worker_dir)
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

    Ok(())
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
pub fn land(project_dir: &Path, id: &str, main_branch: &str) -> Result<(), Error> {
    let worktree_dir = project_dir.join(".worktrees").join(id);
    let worker_dir = worktree_dir.join(".clc").join(WORKER_DIR);

    // Stop worker if still alive.
    if let Some(pid) = read_pid(&worker_dir)
        && is_process_alive(pid)
    {
        stop(project_dir, id)?;
    }

    // Merge the branch.
    merge::merge(project_dir, id, main_branch)?;
    eprintln!("landed '{id}' — merged into trunk");

    Ok(())
}

// --- Helpers ---

fn worker_stdout_path(project_dir: &Path, id: &str) -> std::path::PathBuf {
    project_dir
        .join(".worktrees")
        .join(id)
        .join(".clc")
        .join(WORKER_DIR)
        .join("stdout.jsonl")
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
