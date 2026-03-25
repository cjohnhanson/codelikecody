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

use clc_sdk::agent::Agent;
use clc_sdk::workspace::Workspace;

use camino::Utf8Path;
use nix::sys::stat::Mode;
use nix::unistd::{self, Pid};

use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;
use crate::permissions;
use crate::pickup;

/// Worker state directory name inside `.clc/`.
const WORKER_DIR: &str = "worker";

/// Workspace type for dispatch.
pub enum DispatchWorkspace {
    Worktree,
    Docker {
        image: Option<String>,
        ca: Option<std::sync::Arc<crate::tls::EphemeralCA>>,
        api_port: u16,
        tunnel_port: u16,
    },
}

impl Default for DispatchWorkspace {
    fn default() -> Self {
        Self::Worktree
    }
}

pub fn dispatch(
    project_dir: &Path,
    id: &str,
    main_branch: &str,
    admin_branch: &str,
    model: &str,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    coordinator_id: Option<&str>,
) -> Result<(), Error> {
    dispatch_with_workspace(
        project_dir,
        id,
        main_branch,
        admin_branch,
        model,
        worker_perm_defaults,
        worker_perm_deny,
        coordinator_id,
        &DispatchWorkspace::default(),
    )
}

pub fn dispatch_with_workspace(
    project_dir: &Path,
    id: &str,
    main_branch: &str,
    admin_branch: &str,
    model: &str,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    coordinator_id: Option<&str>,
    workspace_type: &DispatchWorkspace,
) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch, admin_branch)
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

    // Clean up stale worktree from a prior failed run so dispatch is idempotent.
    // A worktree is stale if it exists but has no live worker process.
    if worktree_dir.is_dir() {
        eprintln!("cleaning up stale worktree for '{id}'");
        cleanup_stale_worktree(project_dir, &worktree_dir, id)?;
    } else {
        // Worktree dir doesn't exist but branch might (orphaned from a prior failed dispatch).
        if let Err(e) = crate::gix_ops::delete_branch(project_dir, id) {
            // Branch doesn't exist — fine.
            let _ = e;
        }
    }

    // Pickup creates worktree, sets tisket status, inits clc.
    pickup::pickup(project_dir, id, main_branch, admin_branch, coordinator_id)?;

    // Seed permissions so the worker can function without
    // --dangerously-skip-permissions.
    permissions::seed_defaults(&worktree_dir, worker_perm_defaults, worker_perm_deny)?;

    // Build prompts.
    let initial_prompt = build_worker_prompt(project_dir, id)?;
    let system_prompt = build_system_prompt(id);

    // Spawn the worker.
    let agent = clc_sdk::agent::ClaudeCodeAgent::new();
    let agent_config = clc_sdk::agent::AgentConfig {
        model: model.to_string(),
        system_prompt,
        initial_prompt: initial_prompt.clone(),
        extra_args: vec![],
    };

    match workspace_type {
        DispatchWorkspace::Worktree => {
            let cmd = agent
                .build_start_command(&agent_config, &worktree_dir)
                .map_err(|e| Error::NonBlocking(format!("failed to build agent command: {e}")))?;

            let worker_dir = worktree_dir.join(".clc").join(WORKER_DIR);
            let pid = spawn_agent_process(cmd, &worker_dir, &initial_prompt)?;

            if let Ok(coord) = Coordination::open(project_dir) {
                let _ = coord.register_agent(id, coordinator_id);
                let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Running);
                let _ = coord.set_pid(id, Some(pid.cast_signed()));
            }

            eprintln!("dispatched worker for '{id}' (pid {pid})");
        }
        DispatchWorkspace::Docker { image, ca, api_port, tunnel_port } => {
            use crate::ssh_workspace::{DockerEnvironment, SSHWorkspace, SSHWorkspaceConfig};

            let ws_config = clc_sdk::workspace::WorkspaceConfig {
                tisket_id: id.to_string(),
                project_dir: project_dir.to_path_buf(),
                main_branch: main_branch.to_string(),
                agent_config,
            };

            let image_name = image.clone().unwrap_or_else(|| "clc-worker:latest".to_string());
            let env = DockerEnvironment::new(&image_name, project_dir, id)
                .map_err(|e| Error::NonBlocking(format!("docker env: {e}")))?;

            let oauth_token = std::env::var("CLC_CLAUDE_CODE_OAUTH_TOKEN")
                .ok()
                .or_else(|| {
                    let token_path = dirs::home_dir()?.join(".claude").join("token");
                    std::fs::read_to_string(token_path).ok().map(|s| s.trim().to_string())
                });

            let ssh_config = SSHWorkspaceConfig {
                workspace_config: ws_config,
                ca: ca.clone().unwrap_or_else(|| {
                    std::sync::Arc::new(
                        crate::tls::EphemeralCA::new().expect("ephemeral CA"),
                    )
                }),
                api_port: *api_port,
                oauth_token,
                start_command: None, // Workers use default `clc workspace start`.
            };

            let mut workspace = SSHWorkspace::new(ssh_config, Box::new(env), *tunnel_port)
                .map_err(|e| Error::NonBlocking(format!("ssh workspace: {e}")))?;

            workspace
                .start()
                .map_err(|e| Error::NonBlocking(format!("ssh workspace start: {e}")))?;

            if let Ok(coord) = Coordination::open(project_dir) {
                let _ = coord.register_agent(id, coordinator_id);
                let _ = coord.set_status(id, clc_sdk::coordination::AgentStatus::Running);
            }

            eprintln!("dispatched worker for '{id}' (docker/ssh)");
        }
    }

    Ok(())
}

/// Spawn an agent process with pipe infrastructure.
///
/// Takes a pre-built `Command` from an `Agent::build_start_command` call.
/// Wires up stdin (named pipe), stdout (jsonl file), stderr (log file),
/// writes PID file, and sends the initial prompt.
///
/// Returns the child PID.
pub fn spawn_agent_process(
    mut cmd: Command,
    worker_dir: &Path,
    initial_prompt: &str,
) -> Result<u32, Error> {
    fs::create_dir_all(worker_dir)?;

    let pid_path = worker_dir.join("pid");
    let stdout_path = worker_dir.join("stdout.jsonl");
    let stderr_path = worker_dir.join("stderr.log");
    let stdin_pipe_path = worker_dir.join("stdin.pipe");

    create_named_pipe(&stdin_pipe_path)?;

    let stdout_file = fs::File::create(&stdout_path)?;
    let stderr_file = fs::File::create(&stderr_path)?;

    let stdin_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&stdin_pipe_path)?;

    cmd.stdin(Stdio::from(stdin_file));
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));

    let child = cmd
        .spawn()
        .map_err(|e| Error::NonBlocking(format!("failed to spawn agent: {e}")))?;

    let pid = child.id();
    fs::write(&pid_path, pid.to_string())?;

    send_prompt(&stdin_pipe_path, initial_prompt)?;

    Ok(pid)
}

pub fn is_worker_alive(worktree_dir: &Path) -> bool {
    // Try coordination DB (API or local) — check stored status.
    // worktree_dir is inside .worktrees/<id>/, so project root is two levels up.
    if let Some(project_dir) = worktree_dir.parent().and_then(|p| p.parent()) {
        let has_api = std::env::var("CLC_API_URL").is_ok();
        let has_db = project_dir.join(".clc").join("coordination.db").exists();
        if has_api || has_db {
            if let Ok(coord) = Coordination::open(project_dir) {
                let id = worktree_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                if let Ok(status) = coord.get_status(&id) {
                    return status == clc_sdk::coordination::AgentStatus::Running;
                }
            }
        }
    }

    // Fall back to PID file.
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

/// Clean up a stale worktree left behind by a prior failed dispatch.
///
/// Removes the worktree directory and git metadata, deletes the branch ref,
/// and resets the tisket status back to `todo` so pickup can re-acquire it.
fn cleanup_stale_worktree(project_dir: &Path, worktree_dir: &Path, id: &str) -> Result<(), Error> {
    // Remove worktree directory and .git/worktrees/<id>/ metadata.
    crate::gix_ops::remove_worktree(project_dir, worktree_dir, id)?;

    // Delete the branch ref (may not exist if the prior run failed mid-creation).
    if let Err(e) = crate::gix_ops::delete_branch(project_dir, id) {
        eprintln!("note: branch cleanup for '{id}': {e}");
    }

    // Reset tisket status to todo so pickup can re-acquire it.
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );
    if let Ok(repo) = tisket::Repo::open(utf8_dir)
        && let Ok(issue) = repo.find_issue(id)
        && !issue.frontmatter.status.is_pickable()
        && !issue.frontmatter.status.is_terminal()
    {
        repo.edit_issue(
            id,
            tisket::EditIssueOptions {
                status: Some("todo"),
                ..Default::default()
            },
        )
        .map_err(|e| Error::NonBlocking(format!("failed to reset tisket status: {e}")))?;
        // Commit the status reset on trunk so pickup sees it.
        crate::gix_ops::commit_paths(
            project_dir,
            &format!("clc: reset {id} for re-dispatch"),
            &[".tisket/"],
        )?;
    }

    Ok(())
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

    // Record in coordination DB if available.
    // Walk up from pipe_path (.clc/worker/stdin.pipe) to find the project root.
    if let Some(worker_dir) = pipe_path.parent() {
        if let Some(clc_dir) = worker_dir.parent() {
            if let Some(worktree_dir) = clc_dir.parent() {
                // Check project root (parent of .worktrees/<id>)
                if let Some(project_dir) = worktree_dir.parent().and_then(|p| p.parent()) {
                    let has_api = std::env::var("CLC_API_URL").is_ok();
                    let has_db = project_dir.join(".clc").join("coordination.db").exists();
                    if has_api || has_db {
                        if let Ok(coord) = Coordination::open(project_dir) {
                            let worker_id = worktree_dir
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let msg = clc_sdk::coordination::Message {
                                id: format!(
                                    "input-{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                ),
                                from: "coordinator".into(),
                                to: worker_id,
                                kind: clc_sdk::coordination::MessageKind::Text(
                                    prompt.to_string(),
                                ),
                                timestamp: std::time::SystemTime::now(),
                            };
                            let _ = coord.send(msg);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn build_worker_prompt(project_dir: &Path, tisket_id: &str) -> Result<String, Error> {
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

/// Build worker prompt from the workspace directory itself (for clc workspace start).
pub fn build_worker_prompt_from_dir(workspace_dir: &Path, tisket_id: &str) -> Result<String, Error> {
    build_worker_prompt(workspace_dir, tisket_id)
}

pub fn build_system_prompt(tisket_id: &str) -> String {
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
