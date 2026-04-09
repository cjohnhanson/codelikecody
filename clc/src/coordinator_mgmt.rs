//! Admin coordinator management: list, check, log, send, stop, land.
//!
//! Coordinator state lives in `.clc/coordinators/<id>/` on trunk.
//! This mirrors the worker management interface but for coordinators.

use std::fs;
use std::io::Write;
use std::path::Path;

use claude_code::protocol::{ContentBlock, OutputMessage};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::error::Error;

/// Coordinator state directory inside `.clc/`.
const COORDINATORS_DIR: &str = "coordinators";

struct CoordinatorInfo {
    id: String,
    pid: Option<i32>,
    alive: bool,
    line_count: usize,
    last_activity: String,
}

/// List coordinators. By default only shows live ones; pass `all=true` to include dead.
pub fn list_coordinators(project_dir: &Path, all: bool) -> Result<(), Error> {
    let mut coordinators = collect_coordinators(project_dir)?;

    // Add Docker-hosted coordinators from the coordination DB that have no
    // local state directory. When `clc up` starts coordinators in Docker
    // containers, they exist only in the DB — not in `.clc/coordinators/`.
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_db {
        if let Ok(coord) = crate::coordination::Coordination::open(project_dir) {
            // Get coordinator names from the topology config so we can
            // identify which agents in the DB are coordinators.
            let topo_names: Vec<String> = crate::topology::load(project_dir)
                .ok()
                .flatten()
                .map(|t| t.coordinators.keys().cloned().collect())
                .unwrap_or_default();

            let existing_ids: Vec<String> = coordinators.iter().map(|c| c.id.clone()).collect();
            if let Ok(all_agents) = coord.list_agents(None) {
                for (id, status) in &all_agents {
                    if !topo_names.iter().any(|n| n == id) {
                        continue;
                    }
                    if existing_ids.iter().any(|eid| eid == id.as_str()) {
                        continue;
                    }
                    let is_active = matches!(
                        status,
                        clc_sdk::coordination::AgentStatus::Running
                            | clc_sdk::coordination::AgentStatus::Pending
                    );
                    if !is_active && !all {
                        continue;
                    }
                    coordinators.push(CoordinatorInfo {
                        id: id.clone(),
                        pid: None,
                        alive: is_active,
                        line_count: 0,
                        last_activity: format!("[docker] {status:?}"),
                    });
                }
            }
        }
    }

    let visible: Vec<&CoordinatorInfo> = coordinators.iter().filter(|c| all || c.alive).collect();

    if visible.is_empty() {
        eprintln!("no coordinators");
        return Ok(());
    }

    for c in &visible {
        let status = if c.alive { "working" } else { "dead" };
        let pid_str = c.pid.map_or_else(|| "?".to_string(), |p| p.to_string());
        println!(
            "{}\t{}\tpid={}\tlines={}\t{}",
            c.id, status, pid_str, c.line_count, c.last_activity
        );
    }

    Ok(())
}

/// Collect `CoordinatorInfo` for all entries in `.clc/coordinators/`.
fn collect_coordinators(project_dir: &Path) -> Result<Vec<CoordinatorInfo>, Error> {
    let mut coordinators = Vec::new();

    let coords_dir = project_dir.join(".clc").join(COORDINATORS_DIR);
    if !coords_dir.is_dir() {
        return Ok(coordinators);
    }

    let entries = fs::read_dir(&coords_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_str().unwrap_or("unknown").to_string();

        let pid = read_pid(&path);
        let alive = pid.is_some_and(is_process_alive);
        let line_count = count_lines(&path.join("stdout.jsonl"));
        let last_activity = last_activity_summary(&path.join("stdout.jsonl"));

        coordinators.push(CoordinatorInfo {
            id,
            pid,
            alive,
            line_count,
            last_activity,
        });
    }

    Ok(coordinators)
}

/// Resolve the coordinator state directory for a given ID.
fn coordinator_dir(project_dir: &Path, id: &str) -> std::path::PathBuf {
    project_dir.join(".clc").join(COORDINATORS_DIR).join(id)
}

/// Show activity since last check (cursor-based).
pub fn check(project_dir: &Path, id: &str) -> Result<(), Error> {
    let coord_dir = coordinator_dir(project_dir, id);
    let stdout_path = coord_dir.join("stdout.jsonl");
    if !stdout_path.exists() {
        return Err(Error::NonBlocking(format!(
            "no coordinator output for '{id}'"
        )));
    }

    // Read cursor.
    let cursor_path = cursor_path(project_dir, id);
    let cursor = read_cursor(&cursor_path);

    // Read lines from cursor.
    let lines = read_stdout_lines(&stdout_path)?;
    let new_lines = &lines[cursor.min(lines.len())..];

    if new_lines.is_empty() {
        eprintln!("no new activity for coordinator '{id}'");
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
    let coord_dir = coordinator_dir(project_dir, id);
    let stdout_path = coord_dir.join("stdout.jsonl");
    if !stdout_path.exists() {
        return Err(Error::NonBlocking(format!(
            "no coordinator output for '{id}'"
        )));
    }

    let lines = read_stdout_lines(&stdout_path)?;
    let start = lines.len().saturating_sub(max_lines);

    for line in &lines[start..] {
        print_parsed_line(line);
    }

    Ok(())
}

/// Send a follow-up message to the coordinator via the named pipe.
pub fn send(project_dir: &Path, id: &str, message: &str) -> Result<(), Error> {
    let coord_dir = coordinator_dir(project_dir, id);
    let pipe_path = coord_dir.join("stdin.pipe");

    // Check the coordinator is alive.
    let pid = read_pid(&coord_dir)
        .ok_or_else(|| Error::NonBlocking(format!("no PID file for coordinator '{id}'")))?;
    if !is_process_alive(pid) {
        return Err(Error::NonBlocking(format!(
            "coordinator '{id}' is not running (pid {pid})"
        )));
    }

    if !pipe_path.exists() {
        return Err(Error::NonBlocking(format!(
            "no stdin pipe for coordinator '{id}'"
        )));
    }

    let input = claude_code::protocol::InputMessage::user(message);
    let json = serde_json::to_string(&input)?;

    let mut file = fs::OpenOptions::new().write(true).open(&pipe_path)?;
    writeln!(file, "{json}")?;
    file.flush()?;

    eprintln!("sent message to coordinator '{id}'");
    Ok(())
}

/// Stop the coordinator process.
pub fn stop(project_dir: &Path, id: &str) -> Result<(), Error> {
    let coord_dir = coordinator_dir(project_dir, id);

    let pid = read_pid(&coord_dir)
        .ok_or_else(|| Error::NonBlocking(format!("no PID file for coordinator '{id}'")))?;

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

        eprintln!("stopped coordinator '{id}' (pid {pid})");
    } else {
        eprintln!("coordinator '{id}' already dead (pid {pid})");
    }

    Ok(())
}

/// Land a coordinator: squash-merge its integration branch into main, clean up registration.
pub fn land(project_dir: &Path, id: &str, main_branch: &str) -> Result<(), Error> {
    let coord_dir = coordinator_dir(project_dir, id);

    if !coord_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no coordinator registration for '{id}'"
        )));
    }

    // Stop coordinator if still alive.
    if let Some(pid) = read_pid(&coord_dir)
        && is_process_alive(pid)
    {
        stop(project_dir, id)?;
    }

    // Read the integration branch name.
    let branch_path = coord_dir.join("branch");
    let branch = fs::read_to_string(&branch_path)
        .map_err(|_| {
            Error::NonBlocking(format!(
                "no branch file for coordinator '{id}' — cannot determine integration branch"
            ))
        })?
        .trim()
        .to_string();

    if branch.is_empty() {
        return Err(Error::NonBlocking(format!(
            "empty branch file for coordinator '{id}'"
        )));
    }

    // Switch to the integration branch, then land it.
    // Use the integrate module to squash-merge onto main.
    crate::gix_ops::checkout_branch(project_dir, &branch)?;
    crate::integrate::land(project_dir, main_branch)?;

    // Clean up coordinator registration.
    fs::remove_dir_all(&coord_dir)?;

    eprintln!("landed coordinator '{id}' — integration branch merged into {main_branch}");

    Ok(())
}

// --- Helpers ---

fn cursor_path(project_dir: &Path, id: &str) -> std::path::PathBuf {
    project_dir
        .join(".clc")
        .join(COORDINATORS_DIR)
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

fn read_pid(coord_dir: &Path) -> Option<i32> {
    let pid_path = coord_dir.join("pid");
    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn is_process_alive(pid: i32) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_dir() -> PathBuf {
        #[allow(deprecated)]
        let dir = tempfile::tempdir().unwrap().into_path();
        dir
    }

    fn create_coordinator_state(coord_dir: &Path, pid: i32) {
        fs::create_dir_all(coord_dir).unwrap();
        fs::write(coord_dir.join("pid"), pid.to_string()).unwrap();
        fs::write(coord_dir.join("stdout.jsonl"), "").unwrap();
    }

    #[test]
    fn coordinator_dir_resolves_correctly() {
        let project = PathBuf::from("/tmp/project");
        let dir = coordinator_dir(&project, "test-coord");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/project/.clc/coordinators/test-coord")
        );
    }

    #[test]
    fn collect_coordinators_finds_registered_coordinator() {
        let project = make_test_dir();

        let coord_dir = project.join(".clc").join("coordinators").join("test-coord");
        create_coordinator_state(&coord_dir, 99999);

        let coordinators = collect_coordinators(&project).unwrap();
        assert_eq!(coordinators.len(), 1);
        assert_eq!(coordinators[0].id, "test-coord");
    }

    #[test]
    fn collect_coordinators_finds_multiple() {
        let project = make_test_dir();

        let coord1 = project.join(".clc").join("coordinators").join("coord-a");
        create_coordinator_state(&coord1, 99999);

        let coord2 = project.join(".clc").join("coordinators").join("coord-b");
        create_coordinator_state(&coord2, 99998);

        let coordinators = collect_coordinators(&project).unwrap();
        assert_eq!(coordinators.len(), 2);

        let ids: Vec<&str> = coordinators.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"coord-a"));
        assert!(ids.contains(&"coord-b"));
    }

    #[test]
    fn collect_coordinators_empty_without_dir() {
        let project = make_test_dir();
        let coordinators = collect_coordinators(&project).unwrap();
        assert!(coordinators.is_empty());
    }

    #[test]
    fn cursor_path_resolves_inside_coordinator_dir() {
        let project = PathBuf::from("/tmp/project");
        let path = cursor_path(&project, "test-coord");
        assert_eq!(
            path,
            PathBuf::from("/tmp/project/.clc/coordinators/test-coord/cursor")
        );
    }
}
