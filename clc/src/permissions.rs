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

#[derive(Debug, Serialize, Deserialize)]
struct PermissionRequest {
    description: String,
    status: String,
}

/// Called by the worker: file a permission request and exit.
///
/// Creates `.clc/worker/permission-request.json` in the current directory
/// (the worker's working directory / worktree root).
pub fn request(cwd: &Path, description: &str) -> Result<(), Error> {
    let request_path = cwd.join(".clc").join("worker").join(REQUEST_FILE);

    if !cwd.join(".clc").join("worker").is_dir() {
        return Err(Error::NonBlocking(
            "no worker state directory found — clc permissions request must be run from within a worker worktree".into(),
        ));
    }

    let req = PermissionRequest {
        description: description.to_string(),
        status: "pending".to_string(),
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
/// Adds the permission to `allowedTools` in the worker's `.claude/settings.local.json`
/// and removes the pending permission request file.
pub fn grant(project_dir: &Path, worker_id: &str, permission: &str) -> Result<(), Error> {
    let work_dir = worker::working_dir_for(project_dir, worker_id);
    let worker_dir = worker::worker_dir_for(project_dir, worker_id);

    if !work_dir.is_dir() {
        return Err(Error::NonBlocking(format!(
            "no working directory for worker '{worker_id}'"
        )));
    }

    // Read (or create) settings.local.json in the worker's worktree.
    let settings_path = work_dir.join(".claude").join("settings.local.json");
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Add to allowedTools array.
    let allowed = settings
        .get_mut("allowedTools")
        .and_then(|v| v.as_array_mut());

    if let Some(arr) = allowed {
        // Avoid duplicates.
        let already_present = arr.iter().any(|v| v.as_str() == Some(permission));
        if !already_present {
            arr.push(serde_json::Value::String(permission.to_string()));
        }
    } else {
        settings["allowedTools"] = serde_json::json!([permission]);
    }

    let json = serde_json::to_string_pretty(&settings)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&settings_path, json)?;

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

/// List pending permission requests across all workers.
///
/// Scans `.worktrees/*/` for workers with a pending permission-request.json.
pub fn list(project_dir: &Path) -> Result<(), Error> {
    let worktrees_dir = project_dir.join(".worktrees");
    if !worktrees_dir.is_dir() {
        eprintln!("no pending permission requests");
        return Ok(());
    }

    let mut found = false;

    for entry in fs::read_dir(&worktrees_dir)? {
        let entry = entry?;
        let worker_id = entry.file_name().to_string_lossy().to_string();
        let request_path = entry.path().join(".clc").join("worker").join(REQUEST_FILE);

        if request_path.exists() {
            let content = fs::read_to_string(&request_path)?;
            if let Ok(req) = serde_json::from_str::<PermissionRequest>(&content)
                && req.status == "pending"
            {
                println!("{worker_id}\t{}", req.description);
                found = true;
            }
        }
    }

    if !found {
        eprintln!("no pending permission requests");
    }

    Ok(())
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

    if req.status == "pending" {
        Some(req.description)
    } else {
        None
    }
}
