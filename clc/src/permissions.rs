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
            "allow": allow,
            "defaultMode": "dontAsk"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_dir() -> PathBuf {
        #[allow(deprecated)]
        tempfile::tempdir().unwrap().into_path()
    }

    // --- seed_baseline tests ---

    #[test]
    fn seed_baseline_creates_settings_file() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        let path = dir.join(".claude").join("settings.local.json");
        assert!(path.exists());
    }

    #[test]
    fn seed_baseline_has_permissions_allow_array() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(!allow.is_empty());
    }

    #[test]
    fn seed_baseline_includes_core_tools() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow: Vec<&str> = settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(allow.contains(&"Read"), "missing Read");
        assert!(allow.contains(&"Write"), "missing Write");
        assert!(allow.contains(&"Edit"), "missing Edit");
        assert!(allow.contains(&"Grep"), "missing Grep");
        assert!(allow.contains(&"Glob"), "missing Glob");
        assert!(allow.contains(&"WebFetch"), "missing WebFetch");
        assert!(allow.contains(&"WebSearch"), "missing WebSearch");
        assert!(allow.contains(&"Bash(clc *)"), "missing Bash(clc *)");
        assert!(allow.contains(&"Bash(cargo *)"), "missing Bash(cargo *)");
    }

    #[test]
    fn seed_baseline_sets_dont_ask_mode() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(settings["permissions"]["defaultMode"], "dontAsk");
    }

    #[test]
    fn seed_baseline_is_idempotent() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        // Modify the file to verify it doesn't get overwritten.
        let path = dir.join(".claude").join("settings.local.json");
        fs::write(&path, r#"{"custom": true}"#).unwrap();

        seed_baseline(&dir).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("custom"),
            "seed_baseline overwrote existing file"
        );
    }

    // --- add_permission_rule tests ---

    #[test]
    fn add_rule_to_empty_file() {
        let dir = make_test_dir();
        let path = dir.join("settings.local.json");
        fs::write(&path, "{}").unwrap();

        add_permission_rule(&path, "Bash(npm *)").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();

        assert_eq!(allow.len(), 1);
        assert_eq!(allow[0], "Bash(npm *)");
    }

    #[test]
    fn add_rule_to_existing_baseline() {
        let dir = make_test_dir();
        seed_baseline(&dir).unwrap();

        let path = dir.join(".claude").join("settings.local.json");
        let before: serde_json::Value = {
            let content = fs::read_to_string(&path).unwrap();
            serde_json::from_str(&content).unwrap()
        };
        let before_count = before["permissions"]["allow"].as_array().unwrap().len();

        add_permission_rule(&path, "Bash(npm *)").unwrap();

        let after: serde_json::Value = {
            let content = fs::read_to_string(&path).unwrap();
            serde_json::from_str(&content).unwrap()
        };
        let after_allow = after["permissions"]["allow"].as_array().unwrap();

        assert_eq!(after_allow.len(), before_count + 1);
        assert!(after_allow.iter().any(|v| v == "Bash(npm *)"));
        // Baseline still present.
        assert!(after_allow.iter().any(|v| v == "Read"));
    }

    #[test]
    fn add_rule_deduplicates() {
        let dir = make_test_dir();
        let path = dir.join("settings.local.json");
        fs::write(&path, "{}").unwrap();

        add_permission_rule(&path, "Bash(npm *)").unwrap();
        add_permission_rule(&path, "Bash(npm *)").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();

        assert_eq!(allow.len(), 1);
    }

    #[test]
    fn add_rule_creates_file_if_missing() {
        let dir = make_test_dir();
        let path = dir.join("settings.local.json");

        add_permission_rule(&path, "Bash(npm *)").unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(settings["permissions"]["allow"][0], "Bash(npm *)");
    }

    // --- request tests ---

    #[test]
    fn request_creates_pending_file() {
        let dir = make_test_dir();
        let wdir = dir.join(".clc").join("worker");
        fs::create_dir_all(&wdir).unwrap();

        request(&dir, "need npm install").unwrap();

        let path = wdir.join(REQUEST_FILE);
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        let req: PermissionRequest = serde_json::from_str(&content).unwrap();
        assert_eq!(req.description, "need npm install");
        assert_eq!(req.status, RequestStatus::Pending);
    }

    #[test]
    fn request_fails_without_worker_dir() {
        let dir = make_test_dir();
        let result = request(&dir, "need something");
        assert!(result.is_err());
    }

    // --- grant tests ---

    #[test]
    fn grant_adds_permission_and_removes_request() {
        let project = make_test_dir();

        // Set up worktree worker structure.
        let worktree = project.join(".worktrees").join("test-worker");
        let worker_dir = worktree.join(".clc").join("worker");
        fs::create_dir_all(&worker_dir).unwrap();
        fs::create_dir_all(worktree.join(".claude")).unwrap();

        // Create a pending request.
        let req = PermissionRequest {
            description: "need npm".into(),
            status: RequestStatus::Pending,
        };
        fs::write(
            worker_dir.join(REQUEST_FILE),
            serde_json::to_string(&req).unwrap(),
        )
        .unwrap();

        grant(&project, "test-worker", "Bash(npm *)").unwrap();

        // Request file removed.
        assert!(!worker_dir.join(REQUEST_FILE).exists());

        // Permission added.
        let settings_path = worktree.join(".claude").join("settings.local.json");
        let content = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v == "Bash(npm *)"));
    }

    #[test]
    fn grant_into_existing_baseline() {
        let project = make_test_dir();

        // Set up worktree with baseline permissions already seeded.
        let worktree = project.join(".worktrees").join("test-worker");
        let worker_dir = worktree.join(".clc").join("worker");
        fs::create_dir_all(&worker_dir).unwrap();
        seed_baseline(&worktree).unwrap();

        // Create a pending request.
        let req = PermissionRequest {
            description: "need npm".into(),
            status: RequestStatus::Pending,
        };
        fs::write(
            worker_dir.join(REQUEST_FILE),
            serde_json::to_string(&req).unwrap(),
        )
        .unwrap();

        grant(&project, "test-worker", "Bash(npm *)").unwrap();

        // New permission added alongside baseline.
        let settings_path = worktree.join(".claude").join("settings.local.json");
        let content = fs::read_to_string(&settings_path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();

        assert!(
            allow.iter().any(|v| v == "Bash(npm *)"),
            "granted permission missing"
        );
        assert!(
            allow.iter().any(|v| v == "Read"),
            "baseline permission lost"
        );
        assert!(
            allow.iter().any(|v| v == "Bash(clc *)"),
            "baseline permission lost"
        );
    }

    #[test]
    fn grant_fails_for_nonexistent_worker() {
        let project = make_test_dir();
        let result = grant(&project, "nonexistent", "Read");
        assert!(result.is_err());
    }

    // --- pending_request tests ---

    #[test]
    fn pending_request_returns_none_without_file() {
        let project = make_test_dir();
        let worktree = project.join(".worktrees").join("test-worker");
        let worker_dir = worktree.join(".clc").join("worker");
        fs::create_dir_all(&worker_dir).unwrap();

        assert!(pending_request(&project, "test-worker").is_none());
    }

    #[test]
    fn pending_request_returns_description() {
        let project = make_test_dir();
        let worker_dir = project
            .join(".worktrees")
            .join("test-worker")
            .join(".clc")
            .join("worker");
        fs::create_dir_all(&worker_dir).unwrap();

        let req = PermissionRequest {
            description: "need docker".into(),
            status: RequestStatus::Pending,
        };
        fs::write(
            worker_dir.join(REQUEST_FILE),
            serde_json::to_string(&req).unwrap(),
        )
        .unwrap();

        assert_eq!(
            pending_request(&project, "test-worker"),
            Some("need docker".into())
        );
    }

    #[test]
    fn pending_request_returns_none_for_granted_status() {
        let project = make_test_dir();
        let worker_dir = project
            .join(".worktrees")
            .join("test-worker")
            .join(".clc")
            .join("worker");
        fs::create_dir_all(&worker_dir).unwrap();

        let req = PermissionRequest {
            description: "was granted".into(),
            status: RequestStatus::Granted,
        };
        fs::write(
            worker_dir.join(REQUEST_FILE),
            serde_json::to_string(&req).unwrap(),
        )
        .unwrap();

        assert!(pending_request(&project, "test-worker").is_none());
    }
}
