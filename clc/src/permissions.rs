//! Permission request system for autonomous workers.
//!
//! Workers call `clc permissions request "description"` to file a permission
//! request and stop. Coordinators call `clc permissions grant <id> <permission>`
//! to approve and add the permission to the worker's settings.local.json.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::coordination::Coordination;
use crate::error::Error;
use crate::worker;

const REQUEST_FILE: &str = "permission-request.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RequestStatus {
    Pending,
    Granted,
    Denied,
}

#[derive(Debug, Serialize, Deserialize)]
struct PermissionRequest {
    description: String,
    status: RequestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    denial_reason: Option<String>,
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
        denial_reason: None,
    };

    let json = serde_json::to_string_pretty(&req)?;
    fs::write(&request_path, json)?;

    // Also record in coordination database if it exists.
    let db_path = cwd.join(".clc").join("coordination.db");
    if db_path.exists() {
    if let Ok(coord) = Coordination::open(cwd) {
        let msg = clc_sdk::coordination::Message {
            id: format!("perm-req-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            from: cwd.file_name().unwrap_or_default().to_string_lossy().to_string(),
            to: "coordinator".into(),
            kind: clc_sdk::coordination::MessageKind::PermissionRequest {
                tool_name: description.to_string(),
                reason: description.to_string(),
            },
            timestamp: std::time::SystemTime::now(),
        };
        let _ = coord.send(msg);
    }}

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

    // Resolve matching escalation if one exists.
    let escalation_path = project_dir
        .join(".clc")
        .join("escalations")
        .join(format!("{worker_id}.json"));
    if escalation_path.exists() {
        fs::remove_file(&escalation_path)?;
    }

    // Record grant in coordination DB.
    let db_path = project_dir.join(".clc").join("coordination.db");
    if db_path.exists() {
        if let Ok(coord) = Coordination::open(project_dir) {
            let msg = clc_sdk::coordination::Message {
                id: format!(
                    "perm-grant-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                from: "coordinator".into(),
                to: worker_id.into(),
                kind: clc_sdk::coordination::MessageKind::PermissionGrant {
                    request_id: format!("perm-req:{worker_id}"),
                    scope: permission.to_string(),
                },
                timestamp: std::time::SystemTime::now(),
            };
            let _ = coord.send(msg);
        }
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

/// Legacy baseline permission seeding. Superseded by `seed_defaults` which supports
/// config-driven permissions and deny rules. Retained for existing tests.
#[cfg(test)]
fn seed_baseline(working_dir: &Path, extra_allow: &[String]) -> Result<(), Error> {
    let settings_path = working_dir.join(".claude").join("settings.local.json");

    // Read existing settings or start fresh.
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Don't overwrite if permissions are already seeded (e.g., re-dispatch after grant).
    if settings
        .get("permissions")
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
        .is_some_and(|arr| !arr.is_empty())
    {
        return Ok(());
    }

    // Build allow list: baseline + project-level extras, deduplicated.
    let mut seen = std::collections::HashSet::new();
    let mut allow: Vec<serde_json::Value> = Vec::new();

    for p in BASELINE_PERMISSIONS {
        if seen.insert(*p) {
            allow.push(serde_json::Value::String(p.to_string()));
        }
    }
    for p in extra_allow {
        if seen.insert(p.as_str()) {
            allow.push(serde_json::Value::String(p.clone()));
        }
    }

    // Merge permissions into existing settings (preserving hooks, etc.).
    settings["permissions"] = serde_json::json!({
        "allow": allow,
        "defaultMode": "dontAsk"
    });

    let json = serde_json::to_string_pretty(&settings)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&settings_path, json)?;

    Ok(())
}

/// Write permissions into a worker's `.claude/settings.local.json` using
/// config-driven defaults and deny rules.
///
/// When `config_defaults` is non-empty, uses those permissions instead of
/// `BASELINE_PERMISSIONS`. When empty, falls back to the hardcoded baseline.
///
/// `config_deny` rules are written to `permissions.deny` in the settings file.
/// `{worktree}` placeholders in both allow and deny lists are expanded to the
/// actual `working_dir` path.
///
/// Idempotent — skips if `permissions.allow` is already present.
pub fn seed_defaults(
    working_dir: &Path,
    config_defaults: &[String],
    config_deny: &[String],
) -> Result<(), Error> {
    let settings_path = working_dir.join(".claude").join("settings.local.json");

    // Read existing settings or start fresh.
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Don't overwrite if permissions are already seeded (e.g., re-dispatch after grant).
    if settings
        .get("permissions")
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
        .is_some_and(|arr| !arr.is_empty())
    {
        return Ok(());
    }

    let working_dir_str = working_dir.to_string_lossy();

    // Build allow list: config defaults or hardcoded baseline.
    let mut seen = std::collections::HashSet::new();
    let mut allow: Vec<serde_json::Value> = Vec::new();

    if config_defaults.is_empty() {
        // Fall back to hardcoded baseline.
        for p in BASELINE_PERMISSIONS {
            if seen.insert(p.to_string()) {
                allow.push(serde_json::Value::String(p.to_string()));
            }
        }
    } else {
        // Use config-driven defaults, expanding {worktree}.
        for p in config_defaults {
            let expanded = p.replace("{worktree}", &working_dir_str);
            if seen.insert(expanded.clone()) {
                allow.push(serde_json::Value::String(expanded));
            }
        }
    }

    // Build deny list, expanding {worktree}.
    let deny: Vec<serde_json::Value> = config_deny
        .iter()
        .map(|p| serde_json::Value::String(p.replace("{worktree}", &working_dir_str)))
        .collect();

    // Merge permissions into existing settings (preserving hooks, etc.).
    let mut perms = serde_json::json!({
        "allow": allow,
        "defaultMode": "dontAsk"
    });
    if !deny.is_empty() {
        perms["deny"] = serde_json::Value::Array(deny);
    }
    settings["permissions"] = perms;

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
/// Checks coordination database first, falls back to filesystem scan.
pub fn list(project_dir: &Path) -> Result<(), Error> {
    // Try coordination database first, if it exists.
    let db_path = project_dir.join(".clc").join("coordination.db");
    if db_path.exists() {
    if let Ok(coord) = Coordination::open(project_dir) {
        if let Ok(pending) = coord.pending_permissions("coordinator") {
            if !pending.is_empty() {
                for msg in &pending {
                    if let clc_sdk::coordination::MessageKind::PermissionRequest {
                        ref tool_name,
                        ref reason,
                    } = msg.kind
                    {
                        println!("{}\t{tool_name}: {reason}", msg.from);
                    }
                }
                return Ok(());
            }
        }
    }}

    // Fall back to filesystem scan.
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

// --- Escalation system ---
// The coordinator escalates permission decisions to the user when a request
// seems dangerous, broad, or unclear. Escalation files live in `.clc/escalations/`
// on trunk. One file per worker (latest wins).

const ESCALATIONS_DIR: &str = "escalations";

#[derive(Debug, Serialize, Deserialize)]
struct Escalation {
    worker_id: String,
    description: String,
}

/// Called by the coordinator: escalate a permission decision to the user.
///
/// Creates `.clc/escalations/{worker_id}.json` on trunk.
pub fn escalate(project_dir: &Path, worker_id: &str, description: &str) -> Result<(), Error> {
    let escalations_dir = project_dir.join(".clc").join(ESCALATIONS_DIR);
    fs::create_dir_all(&escalations_dir)?;

    let escalation = Escalation {
        worker_id: worker_id.to_string(),
        description: description.to_string(),
    };

    let path = escalations_dir.join(format!("{worker_id}.json"));
    let json = serde_json::to_string_pretty(&escalation)?;
    fs::write(&path, json)?;

    // Record escalation in coordination DB.
    let db_path = project_dir.join(".clc").join("coordination.db");
    if db_path.exists() {
        if let Ok(coord) = Coordination::open(project_dir) {
            let msg = clc_sdk::coordination::Message {
                id: format!(
                    "escalation-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                from: "coordinator".into(),
                to: "admin".into(),
                kind: clc_sdk::coordination::MessageKind::PermissionRequest {
                    tool_name: format!("escalation:{worker_id}"),
                    reason: description.to_string(),
                },
                timestamp: std::time::SystemTime::now(),
            };
            let _ = coord.send(msg);
        }
    }

    eprintln!(
        "Escalated to user: worker '{worker_id}' — {description}\n\
         User: run `clc permissions inbox` to review pending escalations."
    );

    Ok(())
}

/// Called by the admin/user: deny a permission escalation.
///
/// Removes the escalation file and updates the worker's permission request
/// to `denied` status with the given reason.
pub fn deny(project_dir: &Path, worker_id: &str, reason: &str) -> Result<(), Error> {
    // Remove the escalation file.
    let escalation_path = project_dir
        .join(".clc")
        .join(ESCALATIONS_DIR)
        .join(format!("{worker_id}.json"));
    if escalation_path.exists() {
        fs::remove_file(&escalation_path)?;
    }

    // Update the worker's permission request to denied status.
    let worker_dir = worker::worker_dir_for(project_dir, worker_id);
    let request_path = worker_dir.join(REQUEST_FILE);
    if request_path.exists() {
        let content = fs::read_to_string(&request_path)?;
        let mut req: PermissionRequest = serde_json::from_str(&content)?;
        req.status = RequestStatus::Denied;
        req.denial_reason = Some(reason.to_string());
        let json = serde_json::to_string_pretty(&req)?;
        fs::write(&request_path, json)?;
    }

    // Record denial in coordination DB.
    let db_path = project_dir.join(".clc").join("coordination.db");
    if db_path.exists() {
        if let Ok(coord) = Coordination::open(project_dir) {
            let msg = clc_sdk::coordination::Message {
                id: format!(
                    "perm-deny-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                from: "admin".into(),
                to: worker_id.into(),
                kind: clc_sdk::coordination::MessageKind::PermissionDenied {
                    request_id: format!("escalation:{worker_id}"),
                    reason: reason.to_string(),
                },
                timestamp: std::time::SystemTime::now(),
            };
            let _ = coord.send(msg);
        }
    }

    eprintln!(
        "Permission denied for worker '{worker_id}': {reason}\n\
         Resume the worker with: `clc worker {worker_id} resume`"
    );

    Ok(())
}

/// Called by the user: view pending escalations from the coordinator.
///
/// Scans `.clc/escalations/` for escalation files and prints them.
pub fn inbox(project_dir: &Path) -> Result<(), Error> {
    let escalations_dir = project_dir.join(".clc").join(ESCALATIONS_DIR);

    if !escalations_dir.is_dir() {
        eprintln!("no pending escalations");
        return Ok(());
    }

    let mut found = false;
    for entry in fs::read_dir(&escalations_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = fs::read_to_string(&path)?;
            if let Ok(esc) = serde_json::from_str::<Escalation>(&content) {
                println!("{}\t{}", esc.worker_id, esc.description);
                found = true;
            }
        }
    }

    if !found {
        eprintln!("no pending escalations");
    }

    Ok(())
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
        seed_baseline(&dir, &[]).unwrap();

        let path = dir.join(".claude").join("settings.local.json");
        assert!(path.exists());
    }

    #[test]
    fn seed_baseline_has_permissions_allow_array() {
        let dir = make_test_dir();
        seed_baseline(&dir, &[]).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(!allow.is_empty());
    }

    #[test]
    fn seed_baseline_includes_core_tools() {
        let dir = make_test_dir();
        seed_baseline(&dir, &[]).unwrap();

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
        seed_baseline(&dir, &[]).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(settings["permissions"]["defaultMode"], "dontAsk");
    }

    #[test]
    fn seed_baseline_is_idempotent() {
        let dir = make_test_dir();
        seed_baseline(&dir, &[]).unwrap();

        let path = dir.join(".claude").join("settings.local.json");

        // Add an extra permission to simulate a prior grant.
        add_permission_rule(&path, "Bash(npm *)").unwrap();
        let before = fs::read_to_string(&path).unwrap();
        let before_count =
            serde_json::from_str::<serde_json::Value>(&before).unwrap()["permissions"]["allow"]
                .as_array()
                .unwrap()
                .len();

        // Second seed should not overwrite since permissions.allow exists.
        seed_baseline(&dir, &[]).unwrap();

        let after = fs::read_to_string(&path).unwrap();
        let after_count = serde_json::from_str::<serde_json::Value>(&after).unwrap()["permissions"]
            ["allow"]
            .as_array()
            .unwrap()
            .len();

        assert_eq!(
            before_count, after_count,
            "seed_baseline overwrote existing permissions"
        );
    }

    #[test]
    fn seed_baseline_merges_into_existing_settings() {
        let dir = make_test_dir();
        let path = dir.join(".claude").join("settings.local.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Write a settings file with hooks but no permissions (what clc init produces).
        fs::write(
            &path,
            r#"{"hooks": {"PreToolUse": [{"hooks": [{"command": "clc hook", "type": "command"}]}]}}"#,
        )
        .unwrap();

        seed_baseline(&dir, &[]).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Hooks preserved.
        assert!(settings.get("hooks").is_some(), "hooks lost during seed");
        // Permissions added.
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(!allow.is_empty(), "permissions not seeded");
        assert!(allow.iter().any(|v| v == "Read"), "missing Read");
    }

    #[test]
    fn seed_baseline_includes_extra_allow() {
        let dir = make_test_dir();
        let extras = vec!["Bash(npm *)".to_string(), "Bash(make *)".to_string()];
        seed_baseline(&dir, &extras).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow: Vec<&str> = settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        // Baseline present.
        assert!(allow.contains(&"Read"), "missing baseline Read");
        // Extras present.
        assert!(allow.contains(&"Bash(npm *)"), "missing extra npm");
        assert!(allow.contains(&"Bash(make *)"), "missing extra make");
    }

    #[test]
    fn seed_baseline_deduplicates_extras() {
        let dir = make_test_dir();
        // "Read" is already in baseline — should not appear twice.
        let extras = vec!["Read".to_string(), "Bash(npm *)".to_string()];
        seed_baseline(&dir, &extras).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();

        let read_count = allow.iter().filter(|v| v.as_str() == Some("Read")).count();
        assert_eq!(read_count, 1, "Read duplicated");
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
        seed_baseline(&dir, &[]).unwrap();

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
            denial_reason: None,
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
        seed_baseline(&worktree, &[]).unwrap();

        // Create a pending request.
        let req = PermissionRequest {
            description: "need npm".into(),
            status: RequestStatus::Pending,
            denial_reason: None,
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
            denial_reason: None,
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
            denial_reason: None,
        };
        fs::write(
            worker_dir.join(REQUEST_FILE),
            serde_json::to_string(&req).unwrap(),
        )
        .unwrap();

        assert!(pending_request(&project, "test-worker").is_none());
    }

    // --- escalation tests ---

    #[test]
    fn escalate_creates_file() {
        let project = make_test_dir();
        fs::create_dir_all(project.join(".clc")).unwrap();

        escalate(&project, "test-worker", "needs docker access").unwrap();

        let path = project
            .join(".clc")
            .join("escalations")
            .join("test-worker.json");
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        let esc: Escalation = serde_json::from_str(&content).unwrap();
        assert_eq!(esc.worker_id, "test-worker");
        assert_eq!(esc.description, "needs docker access");
    }

    #[test]
    fn escalate_overwrites_previous() {
        let project = make_test_dir();
        fs::create_dir_all(project.join(".clc")).unwrap();

        escalate(&project, "test-worker", "first request").unwrap();
        escalate(&project, "test-worker", "second request").unwrap();

        let path = project
            .join(".clc")
            .join("escalations")
            .join("test-worker.json");
        let content = fs::read_to_string(&path).unwrap();
        let esc: Escalation = serde_json::from_str(&content).unwrap();
        assert_eq!(esc.description, "second request");
    }

    #[test]
    fn grant_resolves_escalation() {
        let project = make_test_dir();

        // Set up worktree.
        let worktree = project.join(".worktrees").join("test-worker");
        let worker_dir = worktree.join(".clc").join("worker");
        fs::create_dir_all(&worker_dir).unwrap();
        fs::create_dir_all(worktree.join(".claude")).unwrap();

        // Create an escalation.
        escalate(&project, "test-worker", "needs docker").unwrap();
        let esc_path = project
            .join(".clc")
            .join("escalations")
            .join("test-worker.json");
        assert!(esc_path.exists());

        // Grant resolves the escalation.
        grant(&project, "test-worker", "Bash(docker *)").unwrap();
        assert!(!esc_path.exists(), "escalation not resolved after grant");
    }

    // --- seed_defaults tests (replaces seed_baseline with config-driven permissions) ---

    #[test]
    fn seed_defaults_uses_config_permissions_when_provided() {
        let dir = make_test_dir();
        let config_defaults = vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Write({worktree}/**)".to_string(),
        ];
        let config_deny = vec!["Write({worktree}/.clc/**)".to_string()];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow: Vec<&str> = settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        let dir_str = dir.to_string_lossy();
        let expected_write = format!("Write({dir_str}/**)");

        // Should use config defaults, not hardcoded BASELINE_PERMISSIONS.
        assert!(allow.contains(&"Read"));
        assert!(allow.contains(&"Grep"));
        assert!(allow.contains(&expected_write.as_str()), "Write permission with expanded worktree missing");
        // Should NOT contain hardcoded permissions not in config.
        assert!(!allow.contains(&"WebFetch"), "hardcoded WebFetch should not appear when config provides defaults");
        assert!(!allow.contains(&"MultiEdit"), "hardcoded MultiEdit should not appear when config provides defaults");
    }

    #[test]
    fn seed_defaults_writes_deny_rules() {
        let dir = make_test_dir();
        let config_defaults = vec!["Read".to_string()];
        let config_deny = vec![
            "Write({worktree}/.clc/**)".to_string(),
            "Edit({worktree}/.clc/**)".to_string(),
        ];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let deny = settings["permissions"]["deny"]
            .as_array()
            .expect("permissions.deny should be an array");

        let dir_str = dir.to_string_lossy();
        let expected_write_deny = format!("Write({dir_str}/.clc/**)");
        let expected_edit_deny = format!("Edit({dir_str}/.clc/**)");

        assert_eq!(deny.len(), 2);
        assert!(deny.iter().any(|v| v.as_str() == Some(expected_write_deny.as_str())));
        assert!(deny.iter().any(|v| v.as_str() == Some(expected_edit_deny.as_str())));
    }

    #[test]
    fn seed_defaults_expands_worktree_placeholder() {
        let dir = make_test_dir();
        let config_defaults = vec![
            "Write({worktree}/**)".to_string(),
            "Edit({worktree}/**)".to_string(),
        ];
        let config_deny = vec![
            "Write({worktree}/.clc/**)".to_string(),
        ];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow: Vec<&str> = settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let deny: Vec<&str> = settings["permissions"]["deny"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        let dir_str = dir.to_string_lossy();

        // {worktree} should be expanded to the actual working directory path.
        assert!(
            allow.iter().any(|p| p.contains(&*dir_str) && p.contains("/**")),
            "allow rules should have {{worktree}} expanded to actual path, got: {allow:?}",
        );
        assert!(
            deny.iter().any(|p| p.contains(&*dir_str) && p.contains("/.clc/**")),
            "deny rules should have {{worktree}} expanded to actual path, got: {deny:?}",
        );
        // Should NOT contain the literal {worktree} placeholder.
        assert!(
            !allow.iter().any(|p| p.contains("{worktree}")),
            "allow should not contain literal {{worktree}}"
        );
        assert!(
            !deny.iter().any(|p| p.contains("{worktree}")),
            "deny should not contain literal {{worktree}}"
        );
    }

    #[test]
    fn seed_defaults_falls_back_to_baseline_when_config_empty() {
        let dir = make_test_dir();
        // Empty config defaults = use hardcoded BASELINE_PERMISSIONS.
        let config_defaults: Vec<String> = vec![];
        let config_deny: Vec<String> = vec![];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        let allow: Vec<&str> = settings["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        // Should fall back to baseline permissions.
        assert!(allow.contains(&"Read"), "missing baseline Read");
        assert!(allow.contains(&"Write"), "missing baseline Write");
        assert!(allow.contains(&"Bash(clc *)"), "missing baseline Bash(clc *)");
    }

    #[test]
    fn seed_defaults_is_idempotent() {
        let dir = make_test_dir();
        let config_defaults = vec!["Read".to_string(), "Grep".to_string()];
        let config_deny = vec!["Write({worktree}/.clc/**)".to_string()];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let path = dir.join(".claude").join("settings.local.json");

        // Add an extra permission to simulate a prior grant.
        add_permission_rule(&path, "Bash(npm *)").unwrap();
        let before = fs::read_to_string(&path).unwrap();
        let before_count =
            serde_json::from_str::<serde_json::Value>(&before).unwrap()["permissions"]["allow"]
                .as_array()
                .unwrap()
                .len();

        // Second seed should not overwrite since permissions.allow exists.
        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let after = fs::read_to_string(&path).unwrap();
        let after_count = serde_json::from_str::<serde_json::Value>(&after).unwrap()["permissions"]
            ["allow"]
            .as_array()
            .unwrap()
            .len();

        assert_eq!(
            before_count, after_count,
            "seed_defaults overwrote existing permissions"
        );
    }

    #[test]
    fn seed_defaults_sets_dont_ask_mode() {
        let dir = make_test_dir();
        let config_defaults = vec!["Read".to_string()];
        let config_deny = vec![];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(dir.join(".claude").join("settings.local.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(settings["permissions"]["defaultMode"], "dontAsk");
    }

    #[test]
    fn seed_defaults_merges_into_existing_settings() {
        let dir = make_test_dir();
        let path = dir.join(".claude").join("settings.local.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Write a settings file with hooks but no permissions.
        fs::write(
            &path,
            r#"{"hooks": {"PreToolUse": [{"hooks": [{"command": "clc hook", "type": "command"}]}]}}"#,
        )
        .unwrap();

        let config_defaults = vec!["Read".to_string()];
        let config_deny = vec!["Write({worktree}/.clc/**)".to_string()];

        seed_defaults(&dir, &config_defaults, &config_deny).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Hooks preserved.
        assert!(settings.get("hooks").is_some(), "hooks lost during seed");
        // Permissions added.
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(!allow.is_empty(), "permissions not seeded");
    }
}
