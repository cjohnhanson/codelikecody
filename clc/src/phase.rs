use std::path::Path;

use crate::coordination::Coordination;
use crate::error::Error;
use crate::workflow::Workflow;

const STATE_FILENAME: &str = "state";

/// Load the current phase name as a string.
/// Uses supervisor API when CLC_API_URL is set, falls back to `.clc/state`.
pub fn load_name(project_dir: &Path) -> Result<Option<String>, Error> {
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        return load_phase_name_from_api(&api_url, &agent_id);
    }
    let state = load_state_raw(project_dir)?;
    Ok(state.map(|s| s.phase_name))
}

/// Load the workflow name from state. Returns None if no workflow is stored.
pub fn load_workflow_name(project_dir: &Path) -> Result<Option<String>, Error> {
    let state = load_state_raw(project_dir)?;
    Ok(state.and_then(|s| s.workflow))
}

/// Load the current attempts count from `.clc/state`.
pub fn load_attempts(project_dir: &Path) -> Result<u32, Error> {
    let state = load_state_raw(project_dir)?;
    Ok(state.map_or(0, |s| s.attempts))
}

fn load_phase_name_from_api(api_url: &str, agent_id: &str) -> Result<Option<String>, Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

    let result: Result<serde_json::Value, Error> = rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| Error::NonBlocking(format!("{e}")))?;
        client
            .get(format!("{api_url}/agents/{agent_id}/phase"))
            .send()
            .await
            .map_err(|e| Error::NonBlocking(format!("{e}")))?
            .json()
            .await
            .map_err(|e| Error::NonBlocking(format!("{e}")))
    });

    match result {
        Ok(resp) => {
            let name = resp["phase"].as_str().unwrap_or("tests-unwritten");
            Ok(Some(name.to_string()))
        }
        Err(_) => Ok(None),
    }
}

/// Raw state from the `.clc/state` file.
struct RawState {
    phase_name: String,
    attempts: u32,
    workflow: Option<String>,
}

fn load_state_raw(project_dir: &Path) -> Result<Option<RawState>, Error> {
    let state_path = project_dir.join(".clc").join(STATE_FILENAME);

    if !state_path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&state_path).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to read state {}: {e}",
            state_path.display()
        ))
    })?;

    let phase_name = contents
        .lines()
        .find_map(|line| line.strip_prefix("phase:").map(str::trim))
        .ok_or_else(|| {
            Error::NonBlocking(format!(
                "state file {} missing phase field",
                state_path.display()
            ))
        })?
        .to_string();

    let attempts = contents
        .lines()
        .find_map(|line| line.strip_prefix("attempts:").map(str::trim))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let workflow = contents
        .lines()
        .find_map(|line| line.strip_prefix("workflow:").map(str::trim))
        .map(|s| s.to_string());

    Ok(Some(RawState {
        phase_name,
        attempts,
        workflow,
    }))
}

/// Initialize the phase state for a freshly created worktree.
/// Unlike `set_with_workflow`, this bypasses sequential-transition validation —
/// it is only for use during `clc pickup` when the worktree has no prior state.
pub fn init_phase_with_workflow(
    project_dir: &Path,
    target: &str,
    workflow_name: Option<&str>,
) -> Result<(), Error> {
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        return init_phase_via_api(&api_url, &agent_id, target);
    }

    write_state_str(project_dir, target, 0, workflow_name)
}

/// Validate and perform a phase transition using the given workflow.
/// The workflow graph determines valid transitions.
pub fn set_with_workflow(
    project_dir: &Path,
    target: &str,
    required_attempts: u32,
    workflow: &Workflow,
) -> Result<(), Error> {
    if !workflow.has_phase(target) {
        return Err(Error::NonBlocking(format!(
            "unknown phase '{target}' in the active workflow"
        )));
    }

    let raw_state = load_state_raw(project_dir)?;
    let current_name = raw_state.as_ref().map(|s| s.phase_name.as_str());

    match current_name {
        None => {
            if target != workflow.initial_phase() {
                return Err(Error::NonBlocking(format!(
                    "cannot set phase to '{target}': no current phase, must start with '{}'",
                    workflow.initial_phase()
                )));
            }
        }
        Some(current) => {
            if current == target {
                return Err(Error::NonBlocking(format!(
                    "already at phase '{current}'"
                )));
            }

            if !workflow.is_valid_transition(current, target) {
                return Err(Error::NonBlocking(format!(
                    "cannot transition from '{current}' to '{target}': not a valid transition"
                )));
            }
        }
    }

    let is_forward = current_name.map_or(true, |c| !workflow.is_backward(c, target));

    // Review gating: if this forward transition requires reviews, check for approvals.
    if is_forward {
        if let Some(current) = current_name {
            if let Some(required) = workflow.transition_requires(current, target) {
                let worker_id = crate::git::current_branch(project_dir).unwrap_or_default();
                crate::review::check_review_requirements(project_dir, &worker_id, required)?;
            }
        }
    }

    // Attempt gating: only applies to forward transitions from an existing phase.
    if is_forward && required_attempts > 1 && raw_state.is_some() {
        let current_attempts = raw_state.as_ref().map_or(0, |s| s.attempts);
        let next_attempt = current_attempts + 1;

        if next_attempt < required_attempts {
            let wf_name = raw_state.as_ref().and_then(|s| s.workflow.as_deref());
            write_state_str(
                project_dir,
                current_name.unwrap_or(workflow.initial_phase()),
                next_attempt,
                wf_name,
            )?;
            return Err(Error::NonBlocking(format!(
                "attempt {next_attempt}/{required_attempts} to advance to '{target}': \
                 reconsider before trying again"
            )));
        }
    }

    // Transition succeeds — write new phase with attempts reset.
    let wf_name = raw_state.as_ref().and_then(|s| s.workflow.as_deref());
    write_state_str(project_dir, target, 0, wf_name)?;

    // Record phase transition in coordination database.
    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            let branch = crate::git::current_branch(project_dir).unwrap_or_default();
            let msg = clc_sdk::coordination::Message {
                id: format!(
                    "phase-{}-{}",
                    target,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                from: branch,
                to: "coordinator".into(),
                kind: clc_sdk::coordination::MessageKind::StatusUpdate {
                    phase: target.to_string(),
                    detail: format!("transitioned to {target}"),
                },
                timestamp: std::time::SystemTime::now(),
            };
            let _ = coord.send(msg);
        }
    }

    Ok(())
}

fn write_state_str(
    project_dir: &Path,
    phase: &str,
    attempts: u32,
    workflow: Option<&str>,
) -> Result<(), Error> {
    use std::fmt::Write;

    let clc_dir = project_dir.join(".clc");
    std::fs::create_dir_all(&clc_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to create .clc dir: {e}")))?;
    let state_path = clc_dir.join(STATE_FILENAME);

    // Preserve lines that aren't phase/attempts/workflow (e.g., "untracked: true").
    let existing = if state_path.exists() {
        std::fs::read_to_string(&state_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut content = String::new();
    let _ = writeln!(content, "phase: {phase}");
    if attempts > 0 {
        let _ = writeln!(content, "attempts: {attempts}");
    }
    if let Some(wf) = workflow {
        let _ = writeln!(content, "workflow: {wf}");
    }

    // Carry forward lines that aren't phase, attempts, or workflow.
    for line in existing.lines() {
        if !line.starts_with("phase:")
            && !line.starts_with("attempts:")
            && !line.starts_with("workflow:")
            && !line.is_empty()
        {
            content.push_str(line);
            content.push('\n');
        }
    }

    std::fs::write(&state_path, content).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to write state {}: {e}",
            state_path.display()
        ))
    })
}

/// Initialize phase via the supervisor API. No transition validation.
pub fn init_phase_via_api(api_url: &str, agent_id: &str, target: &str) -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

    rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| Error::NonBlocking(format!("{e}")))?;
        let status = client
            .put(format!("{api_url}/agents/{agent_id}/phase"))
            .json(&serde_json::json!({ "phase": target }))
            .send()
            .await
            .map_err(|e| Error::NonBlocking(format!("{e}")))?
            .status();
        if !status.is_success() {
            return Err(Error::NonBlocking(format!(
                "API init_phase failed: {status}"
            )));
        }
        Ok::<_, Error>(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- API integration tests ---

    use clc_sdk::coordination::CoordinationBackend;

    fn start_test_api(agent_id: &str) -> (String, std::thread::JoinHandle<()>) {
        let agent = agent_id.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let db = clc_sdk::coordination_db::DbBackend::connect("sqlite::memory:")
                    .await
                    .unwrap();
                db.create_tables().await.unwrap();
                db.register_agent(&agent, None).await.unwrap();
                let state = std::sync::Arc::new(crate::supervisor_api::ApiState {
                    db: std::sync::Arc::new(db),
                    project_dir: std::path::PathBuf::from("/tmp"),
                });
                let addr = crate::supervisor_api::start(state, 0, None).await.unwrap();
                tx.send(addr.port()).unwrap();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });
        });
        let port = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("API server did not start in time");
        std::thread::sleep(std::time::Duration::from_millis(50));
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn api_set_phase(base_url: &str, agent_id: &str, phase: &str) -> u16 {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            reqwest::Client::new()
                .put(format!("{base_url}/agents/{agent_id}/phase"))
                .json(&serde_json::json!({ "phase": phase }))
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        })
    }

    #[test]
    fn phase_api_set_and_load_name_roundtrip() {
        let agent = "test-name-roundtrip";
        let (base_url, _handle) = start_test_api(agent);

        api_set_phase(&base_url, agent, "implementing");
        let name = load_phase_name_from_api(&base_url, agent).unwrap();
        assert_eq!(name.as_deref(), Some("implementing"));
    }

    #[test]
    fn init_phase_via_api_sets_without_validation() {
        let agent = "test-init-phase";
        let (base_url, _handle) = start_test_api(agent);

        init_phase_via_api(&base_url, agent, "outline").unwrap();
        let name = load_phase_name_from_api(&base_url, agent).unwrap();
        assert_eq!(name.as_deref(), Some("outline"));
    }

    // --- Workflow-based transition tests ---

    #[test]
    fn set_with_workflow_forward_transition() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: tests-unwritten\n").unwrap();

        let wf = Workflow::default_tdd();
        set_with_workflow(dir.path(), "tests-written", 1, &wf).unwrap();
        let name = load_name(dir.path()).unwrap();
        assert_eq!(name.as_deref(), Some("tests-written"));
    }

    #[test]
    fn set_with_workflow_backward_transition() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: implementing\n").unwrap();

        let wf = Workflow::default_tdd();
        set_with_workflow(dir.path(), "tests-unwritten", 1, &wf).unwrap();
        let name = load_name(dir.path()).unwrap();
        assert_eq!(name.as_deref(), Some("tests-unwritten"));
    }

    #[test]
    fn set_with_workflow_rejects_invalid_transition() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: tests-unwritten\n").unwrap();

        let wf = Workflow::default_tdd();
        let result = set_with_workflow(dir.path(), "implementing", 1, &wf);
        assert!(result.is_err());
    }

    #[test]
    fn set_with_workflow_rejects_unknown_phase() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: tests-unwritten\n").unwrap();

        let wf = Workflow::default_tdd();
        let result = set_with_workflow(dir.path(), "nonexistent", 1, &wf);
        assert!(result.is_err());
    }

    #[test]
    fn set_with_workflow_rejects_same_phase() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: implementing\n").unwrap();

        let wf = Workflow::default_tdd();
        let result = set_with_workflow(dir.path(), "implementing", 1, &wf);
        assert!(result.is_err());
    }

    // --- Workflow name in state ---

    #[test]
    fn init_phase_with_workflow_stores_workflow_name() {
        let dir = tempfile::tempdir().unwrap();
        init_phase_with_workflow(dir.path(), "outline", Some("docs")).unwrap();

        let wf_name = load_workflow_name(dir.path()).unwrap();
        assert_eq!(wf_name.as_deref(), Some("docs"));

        let phase = load_name(dir.path()).unwrap();
        assert_eq!(phase.as_deref(), Some("outline"));
    }

    #[test]
    fn init_phase_without_workflow_name() {
        let dir = tempfile::tempdir().unwrap();
        init_phase_with_workflow(dir.path(), "tests-unwritten", None).unwrap();

        let wf_name = load_workflow_name(dir.path()).unwrap();
        assert!(wf_name.is_none());
    }

    #[test]
    fn workflow_name_preserved_through_set_with_workflow() {
        let dir = tempfile::tempdir().unwrap();
        init_phase_with_workflow(dir.path(), "tests-unwritten", Some("tdd")).unwrap();

        let wf = Workflow::default_tdd();
        set_with_workflow(dir.path(), "tests-written", 1, &wf).unwrap();

        let wf_name = load_workflow_name(dir.path()).unwrap();
        assert_eq!(wf_name.as_deref(), Some("tdd"));
    }

    #[test]
    fn load_name_reads_custom_phase() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: outline\n").unwrap();

        let name = load_name(dir.path()).unwrap();
        assert_eq!(name.as_deref(), Some("outline"));
    }

    #[test]
    fn load_attempts_from_state() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: red\nattempts: 3\n").unwrap();

        let attempts = load_attempts(dir.path()).unwrap();
        assert_eq!(attempts, 3);
    }

    #[test]
    fn state_preserves_extra_lines() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: red\nuntracked: true\n").unwrap();

        let wf = Workflow::default_tdd();
        set_with_workflow(dir.path(), "implementing", 1, &wf).unwrap();

        let contents = std::fs::read_to_string(clc_dir.join("state")).unwrap();
        assert!(contents.contains("untracked: true"));
        assert!(contents.contains("phase: implementing"));
    }
}
