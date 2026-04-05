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

/// Load the workflow name from state.
/// Uses supervisor API when CLC_API_URL is set, falls back to `.clc/state`.
pub fn load_workflow_name(project_dir: &Path) -> Result<Option<String>, Error> {
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        return load_workflow_name_from_api(&api_url, &agent_id);
    }
    let state = load_state_raw(project_dir)?;
    Ok(state.and_then(|s| s.workflow))
}

/// Load the current attempts count.
/// Uses supervisor API when CLC_API_URL is set, falls back to `.clc/state`.
pub fn load_attempts(project_dir: &Path) -> Result<u32, Error> {
    if let Ok(api_url) = std::env::var("CLC_API_URL") {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        return load_attempts_from_api(&api_url, &agent_id);
    }
    let state = load_state_raw(project_dir)?;
    Ok(state.map_or(0, |s| s.attempts))
}

fn load_workflow_name_from_api(api_url: &str, agent_id: &str) -> Result<Option<String>, Error> {
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
        Ok(resp) => Ok(resp["workflow"].as_str().map(str::to_string)),
        Err(_) => Ok(None),
    }
}

fn load_attempts_from_api(api_url: &str, agent_id: &str) -> Result<u32, Error> {
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
        Ok(resp) => Ok(resp["attempts"].as_u64().unwrap_or(0) as u32),
        Err(_) => Ok(0),
    }
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
        return init_phase_via_api(&api_url, &agent_id, target, workflow_name);
    }

    write_state_str(project_dir, target, 0, workflow_name)
}

/// Validate and perform a phase transition using the given workflow.
/// The workflow graph determines valid transitions.
///
/// Validate a phase transition against the workflow graph and review gates.
/// Pure logic — no storage reads or writes.
pub fn validate_transition(
    workflow: &Workflow,
    current: Option<&str>,
    target: &str,
    worker_id: &str,
    review_checker: &dyn Fn(&str, &[String]) -> Result<(), Error>,
) -> Result<(), Error> {
    if !workflow.has_phase(target) {
        return Err(Error::NonBlocking(format!(
            "unknown phase '{target}' in the active workflow"
        )));
    }

    match current {
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

            // Review gate: if this is a forward transition with reviewers, check approvals.
            let is_forward = !workflow.is_backward(current, target);
            if is_forward {
                if let Some(reviewers) = workflow.transition_reviewers(current, target) {
                    review_checker(worker_id, reviewers)?;
                }
            }
        }
    }

    Ok(())
}

/// When `CLC_API_URL` is set (Docker workers), phase state is read from
/// and written to the supervisor API. Otherwise uses `.clc/state`.
pub fn set_with_workflow(
    project_dir: &Path,
    target: &str,
    required_attempts: u32,
    workflow: &Workflow,
) -> Result<(), Error> {
    // Load current phase — from API when in a workspace, from filesystem otherwise.
    let use_api = std::env::var("CLC_API_URL").ok();
    let (current_phase, current_attempts, wf_name_owned) = if let Some(ref api_url) = use_api {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        let name = load_phase_name_from_api(api_url, &agent_id)?;
        (name, 0u32, None::<String>)
    } else {
        let raw_state = load_state_raw(project_dir)?;
        (
            raw_state.as_ref().map(|s| s.phase_name.clone()),
            raw_state.as_ref().map_or(0, |s| s.attempts),
            raw_state.as_ref().and_then(|s| s.workflow.clone()),
        )
    };
    let current_name = current_phase.as_deref();

    // Validate the transition using shared logic.
    let worker_id = crate::git::current_branch(project_dir).unwrap_or_default();
    let project_dir_owned = project_dir.to_path_buf();
    validate_transition(
        workflow,
        current_name,
        target,
        &worker_id,
        &|wid, reviewers| {
            crate::review::check_review_requirements(&project_dir_owned, wid, reviewers)
        },
    )?;

    let is_forward = current_name.map_or(true, |c| !workflow.is_backward(c, target));

    // Attempt gating: only applies to forward transitions from an existing phase.
    if is_forward && required_attempts > 1 && current_name.is_some() {
        let next_attempt = current_attempts + 1;

        if next_attempt < required_attempts {
            if use_api.is_some() {
                // API mode: can't store attempts, just reject.
                return Err(Error::NonBlocking(format!(
                    "attempt {next_attempt}/{required_attempts} to advance to '{target}': \
                     reconsider before trying again"
                )));
            } else {
                let wf_name = wf_name_owned.as_deref();
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
    }

    // Transition succeeds — write new phase (preserving workflow name).
    if let Some(ref api_url) = use_api {
        let agent_id = crate::git::current_branch(project_dir).unwrap_or_default();
        init_phase_via_api(api_url, &agent_id, target, wf_name_owned.as_deref())?;
    } else {
        let wf_name = wf_name_owned.as_deref();
        write_state_str(project_dir, target, 0, wf_name)?;
    }

    // Record phase transition in coordination database.
    let has_api = use_api.is_some();
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
pub fn init_phase_via_api(
    api_url: &str,
    agent_id: &str,
    target: &str,
    workflow: Option<&str>,
) -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::NonBlocking(format!("tokio: {e}")))?;

    rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| Error::NonBlocking(format!("{e}")))?;
        let mut body = serde_json::json!({ "phase": target });
        if let Some(wf) = workflow {
            body["workflow"] = serde_json::json!(wf);
        }
        let status = client
            .put(format!("{api_url}/agents/{agent_id}/phase"))
            .json(&body)
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
                    workflows: Default::default(),
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

        // Walk through valid transitions to reach implementing.
        api_set_phase(&base_url, agent, "tests-written");
        api_set_phase(&base_url, agent, "red");
        api_set_phase(&base_url, agent, "implementing");
        let name = load_phase_name_from_api(&base_url, agent).unwrap();
        assert_eq!(name.as_deref(), Some("implementing"));
    }

    #[test]
    fn init_phase_via_api_sets_without_validation() {
        let agent = "test-init-phase";
        let (base_url, _handle) = start_test_api(agent);

        init_phase_via_api(&base_url, agent, "outline", Some("docs")).unwrap();
        let name = load_phase_name_from_api(&base_url, agent).unwrap();
        assert_eq!(name.as_deref(), Some("outline"));
    }

    #[test]
    fn init_phase_via_api_stores_workflow_name() {
        let agent = "test-workflow-store";
        let (base_url, _handle) = start_test_api(agent);

        init_phase_via_api(&base_url, agent, "tests-unwritten", Some("tdd")).unwrap();

        // Verify workflow name is returned by the API.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let body: serde_json::Value = rt.block_on(async {
            reqwest::Client::new()
                .get(format!("{base_url}/agents/{agent}/phase"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
        });
        assert_eq!(body["phase"].as_str(), Some("tests-unwritten"));
        assert_eq!(body["workflow"].as_str(), Some("tdd"));
    }

    #[test]
    fn init_phase_via_api_without_workflow_returns_null() {
        let agent = "test-no-workflow";
        let (base_url, _handle) = start_test_api(agent);

        init_phase_via_api(&base_url, agent, "tests-unwritten", None).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let body: serde_json::Value = rt.block_on(async {
            reqwest::Client::new()
                .get(format!("{base_url}/agents/{agent}/phase"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
        });
        assert_eq!(body["phase"].as_str(), Some("tests-unwritten"));
        assert!(body["workflow"].is_null(), "workflow should be null when not set");
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

    // --- validate_transition unit tests ---

    fn noop_review_checker(_: &str, _: &[String]) -> Result<(), Error> {
        Ok(())
    }

    fn blocking_review_checker(_: &str, reviewers: &[String]) -> Result<(), Error> {
        Err(Error::NonBlocking(format!(
            "review required: {}",
            reviewers.join(", ")
        )))
    }

    #[test]
    fn validate_transition_rejects_unknown_phase() {
        let wf = Workflow::default_tdd();
        let result = validate_transition(&wf, None, "nonexistent", "w", &noop_review_checker);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown phase"));
    }

    #[test]
    fn validate_transition_rejects_skip_forward() {
        let wf = Workflow::default_tdd();
        let result = validate_transition(
            &wf, Some("tests-unwritten"), "implementing", "w", &noop_review_checker,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a valid transition"));
    }

    #[test]
    fn validate_transition_allows_valid_forward() {
        let wf = Workflow::default_tdd();
        let result = validate_transition(
            &wf, Some("tests-unwritten"), "tests-written", "w", &noop_review_checker,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_transition_allows_backward() {
        let wf = Workflow::default_tdd();
        let result = validate_transition(
            &wf, Some("implementing"), "tests-unwritten", "w", &noop_review_checker,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_transition_rejects_same_phase() {
        let wf = Workflow::default_tdd();
        let result = validate_transition(
            &wf, Some("implementing"), "implementing", "w", &noop_review_checker,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already at"));
    }

    #[test]
    fn validate_transition_checks_review_gate() {
        // Build a workflow with a review-gated transition.
        use crate::config::{PhaseDef, TransitionDef, WorkflowDef};
        let wf = Workflow::new(&WorkflowDef {
            description: None,
            phases: vec![
                PhaseDef {
                    name: "writing".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Rich {
                        target: "done".into(),
                        review: vec!["reviewer-a".into()],
                    }]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
            ],
        })
        .unwrap();

        // With blocking checker — should fail.
        let result = validate_transition(
            &wf, Some("writing"), "done", "w", &blocking_review_checker,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("review required"));

        // With noop checker — should pass.
        let result = validate_transition(
            &wf, Some("writing"), "done", "w", &noop_review_checker,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_transition_skips_review_for_backward() {
        use crate::config::{PhaseDef, TransitionDef, WorkflowDef};
        let wf = Workflow::new(&WorkflowDef {
            description: None,
            phases: vec![
                PhaseDef {
                    name: "writing".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Rich {
                        target: "done".into(),
                        review: vec!["reviewer-a".into()],
                    }]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Simple("writing".into())]),
                },
            ],
        })
        .unwrap();

        // Backward from done → writing should NOT check reviews.
        let result = validate_transition(
            &wf, Some("done"), "writing", "w", &blocking_review_checker,
        );
        assert!(result.is_ok(), "backward transition should skip review gate");
    }
}
