//! Permission request system for autonomous workers.
//!
//! Workers call `clc permissions request "description"` to file a permission
//! request and stop. Coordinators call `clc permissions grant <id> <permission>`
//! to approve and add the permission to the worker's settings.local.json.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::worker;

const REQUEST_FILE: &str = "permission-request.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RequestStatus {
    Pending,
    Granted,
}

#[derive(Debug, Serialize, Deserialize)]
struct PermissionRequest {
    description: String,
    status: RequestStatus,
}

/// Called by the worker: file a permission request and exit.
///
/// Creates `permission-request.json` in the worker's state directory.
/// `cwd` is the worker's current working directory (worktree root or trunk).
pub fn request(cwd: &Path, description: &str) -> Result<(), Error> {
    // The worker state dir is always at `.clc/worker/` relative to cwd,
    // whether we're in a worktree or on trunk as the coordinator.
    let wdir = cwd.join(".clc").join("worker");
    let request_path = wdir.join(REQUEST_FILE);

    if !wdir.is_dir() {
        return Err(Error::NonBlocking(
            "no worker state directory found — must be run from within a worker".into(),
        ));
    }

    let req = PermissionRequest {
        description: description.to_string(),
        status: RequestStatus::Pending,
    };

    let json = serde_json::to_string_pretty(&req)?;
    fs::write(&request_path, json)?;

    eprintln!(
        "Permission request filed: \"{description}\"\n\
         Stopping to await coordinator approval. \
         Coordinator: run `clc permissions grant <worker-id> <permission>` to approve."
    );

    Ok(())
}

/// Called by the coordinator: grant a permission to a worker.
///
/// Adds the permission to `permissions.allow` in the worker's `.claude/settings.local.json`
/// and removes the pending permission request file.
pub fn grant(project_dir: &Path, worker_id: &str, permission: &str) -> Result<(), Error> {
    let work_dir = worker::working_dir_for(project_dir, worker_id);
    let worker_dir = worker::worker_dir_for(project_dir, worker_id);

    if !work_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no working directory for worker '{worker_id}'"
        )));
    }

    let settings_path = work_dir.join(".claude").join("settings.local.json");
    add_permission_rule(&settings_path, permission)?;

    // Remove the pending request file.
    let request_path = worker_dir.join(REQUEST_FILE);
    if request_path.exists() {
        fs::remove_file(&request_path)?;
    }

    eprintln!(
        "Permission granted for worker '{worker_id}': {permission}\n\
         Resume the worker with: `clc worker {worker_id} resume`"
    );

    Ok(())
}

/// Baseline permissions pre-seeded into every worker's `.claude/settings.local.json`
/// at dispatch time. These cover the tools workers need to function within the clc
/// workflow without hitting permission prompts for routine operations.
const BASELINE_PERMISSIONS: &[&str] = &[
    // File operations — workers need to read, write, and edit code.
    "Read",
    "Write",
    "Edit",
    "MultiEdit",
    // Search tools.
    "Grep",
    "Glob",
    // Web tools for documentation lookup.
    "WebFetch",
    "WebSearch",
    // clc workflow commands.
    "Bash(clc *)",
    // Tisket issue tracker commands.
    "Bash(tisket *)",
    // Missouri test runner.
    "Bash(missouri *)",
    // Cargo build and test.
    "Bash(cargo *)",
    // Git operations (staging, committing — not push).
    "Bash(git add *)",
    "Bash(git commit *)",
    "Bash(git status *)",
    "Bash(git diff *)",
    "Bash(git log *)",
    "Bash(git show *)",
    // Basic filesystem commands.
    "Bash(ls *)",
    "Bash(mkdir *)",
    "Bash(cat *)",
    "Bash(head *)",
    "Bash(tail *)",
    "Bash(wc *)",
    "Bash(find *)",
    "Bash(test *)",
];

/// Write baseline permissions into a worker's `.claude/settings.local.json`.
///
/// Called at dispatch time so the worker can function without `--dangerously-skip-permissions`.
/// Only writes if the file doesn't already exist (idempotent for re-dispatch after resume).
pub fn seed_baseline(working_dir: &Path) -> Result<(), Error> {
    let settings_path = working_dir.join(".claude").join("settings.local.json");

    // Don't overwrite if already seeded (e.g., re-dispatch after grant).
    if settings_path.exists() {
        return Ok(());
    }

    let allow: Vec<serde_json::Value> = BASELINE_PERMISSIONS
        .iter()
        .map(|p| serde_json::Value::String(p.to_string()))
        .collect();

    let settings = serde_json::json!({
        "permissions": {
            "allow": allow
        }
    });

    let json = serde_json::to_string_pretty(&settings)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&settings_path, json)?;

    Ok(())
}

/// Add a permission rule to `permissions.allow` in a settings.local.json file.
fn add_permission_rule(settings_path: &Path, permission: &str) -> Result<(), Error> {
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Ensure permissions.allow array exists.
    if settings.get("permissions").is_none() {
        settings["permissions"] = serde_json::json!({});
    }

    let perms = settings.get_mut("permissions").unwrap();
    let allowed = perms.get_mut("allow").and_then(|v| v.as_array_mut());

    if let Some(arr) = allowed {
        let already_present = arr.iter().any(|v| v.as_str() == Some(permission));
        if !already_present {
            arr.push(serde_json::Value::String(permission.to_string()));
        }
    } else {
        perms["allow"] = serde_json::json!([permission]);
    }

    let json = serde_json::to_string_pretty(&settings)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(settings_path, json)?;

    Ok(())
}

/// List pending permission requests across all workers.
///
/// Checks coordinator on trunk and all worktree workers.
pub fn list(project_dir: &Path) -> Result<(), Error> {
    let mut found = false;

    // Check coordinator on trunk.
    found |= print_pending(project_dir, worker::COORDINATOR_ID);

    // Check worktree workers.
    let worktrees_dir = project_dir.join(".worktrees");
    if worktrees_dir.is_dir() {
        for entry in fs::read_dir(&worktrees_dir)? {
            let entry = entry?;
            let worker_id = entry.file_name().to_string_lossy().to_string();
            found |= print_pending(project_dir, &worker_id);
        }
    }

    if !found {
        eprintln!("no pending permission requests");
    }

    Ok(())
}

fn print_pending(project_dir: &Path, worker_id: &str) -> bool {
    pending_request(project_dir, worker_id).is_some_and(|description| {
        println!("{worker_id}\t{description}");
        true
    })
}

/// Return the pending permission request for a worker, if any.
pub fn pending_request(project_dir: &Path, worker_id: &str) -> Option<String> {
    let worker_dir = worker::worker_dir_for(project_dir, worker_id);
    let request_path = worker_dir.join(REQUEST_FILE);

    if !request_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&request_path).ok()?;
    let req: PermissionRequest = serde_json::from_str(&content).ok()?;

    if req.status == RequestStatus::Pending {
        Some(req.description)
    } else {
        None
    }
}
