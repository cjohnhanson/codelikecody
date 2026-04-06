use std::path::Path;

use camino::Utf8Path;

use crate::config;
use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;
use crate::phase;
use crate::workflow::Workflow;

pub fn done(project_dir: &Path, main_branch: &str, admin_branch: &str) -> Result<(), Error> {
    // Must not be on main branch.
    let git_state = git::detect(project_dir, main_branch, admin_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if git_state.is_main {
        return Err(Error::NonBlocking(
            "cannot run 'done' on the main branch".into(),
        ));
    }

    // Resolve workflow and check that current phase is terminal.
    let cfg = config::load(project_dir).unwrap_or_default();
    let workflow = resolve_done_workflow(project_dir, &cfg);

    let phase_name = phase::load_name(project_dir)?
        .ok_or_else(|| Error::NonBlocking("no phase set — nothing to finalize".into()))?;

    if !workflow.is_terminal(&phase_name) {
        return Err(Error::NonBlocking(format!(
            "phase must be terminal to finalize, currently '{phase_name}'"
        )));
    }

    // Run test command if configured. Tests must pass to finalize.
    if let Some(ref cmd) = cfg.test_command {
        eprintln!("running test command: {cmd}");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(project_dir)
            .status()
            .map_err(|e| Error::NonBlocking(format!("test command failed to start: {e}")))?;
        if !status.success() {
            return Err(Error::NonBlocking(format!(
                "cannot finalize: test command failed (exit {})",
                status.code().unwrap_or(-1)
            )));
        }
    }

    // Refuse to finalize if the working tree has uncommitted changes outside of
    // ephemeral directories (.clc/ and .claude/).
    if crate::gix_ops::has_relevant_uncommitted_changes(project_dir)? {
        return Err(Error::NonBlocking(
            "working tree has uncommitted changes — commit your work before running 'clc done'"
                .into(),
        ));
    }

    // Close the tisket issue (branch name = issue ID) and commit the change.
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    if let Ok(repo) = tisket::Repo::open(utf8_dir) {
        let issue_id = &git_state.branch;
        match repo.close_issue(issue_id, Some("done")) {
            Ok(()) => {
                let msg = format!("clc: finalize {}", git_state.branch);
                crate::gix_ops::commit_paths(project_dir, &msg, &[".tisket/"])?;
            }
            Err(tisket::Error::IssueNotFound(_)) => {}
            Err(tisket::Error::IssueAlreadyClosed(_)) => {
                return Err(Error::NonBlocking(format!(
                    "already done — tisket '{issue_id}' is already closed"
                )));
            }
            Err(e) => {
                return Err(Error::NonBlocking(format!(
                    "failed to close tisket '{issue_id}': {e}"
                )));
            }
        }
    }

    // Mark agent as completed in coordination database if available.
    let has_api = std::env::var("CLC_API_URL").is_ok();
    let has_db = project_dir.join(".clc").join("coordination.db").exists();
    if has_api || has_db {
        if let Ok(coord) = Coordination::open(project_dir) {
            let _ = coord.set_status(
                &git_state.branch,
                clc_sdk::coordination::AgentStatus::Completed,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix::prelude::Write as _;

    /// Create a temporary git repo on a named branch with an initial commit.
    fn make_repo_on_branch(branch: &str) -> std::path::PathBuf {
        #[allow(deprecated)]
        let dir = tempfile::tempdir().unwrap().into_path();

        gix::init(&dir).unwrap();

        // Set git identity.
        let config_path = dir.join(".git").join("config");
        let mut config = std::fs::read_to_string(&config_path).unwrap_or_default();
        config.push_str("[user]\n\tname = test\n\temail = test@test\n");
        std::fs::write(&config_path, config).unwrap();

        let repo = gix::open(&dir).unwrap();

        // Create initial commit.
        let empty_tree = repo.write(&gix::objs::Tree::empty()).unwrap();
        repo.commit("HEAD", "initial", empty_tree, gix::commit::NO_PARENT_IDS)
            .unwrap();

        // Create and checkout target branch if not main.
        if branch != "main" {
            let head_id = repo.head_id().unwrap().detach();
            let ref_name = format!("refs/heads/{branch}");
            repo.reference(
                ref_name.clone(),
                head_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "create branch",
            )
            .unwrap();

            // Point HEAD at the new branch.
            let head_ref_path = dir.join(".git").join("HEAD");
            std::fs::write(&head_ref_path, format!("ref: {ref_name}\n")).unwrap();
        }

        dir
    }

    #[test]
    fn done_rejects_on_main_branch() {
        let dir = make_repo_on_branch("main");
        let err = done(&dir, "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("main branch"),
            "expected main branch error, got: {err}"
        );
    }

    #[test]
    fn done_rejects_when_no_phase_set() {
        let dir = make_repo_on_branch("feature-123");
        let err = done(&dir, "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("no phase set"),
            "expected no-phase error, got: {err}"
        );
    }

    #[test]
    fn done_rejects_non_terminal_phase() {
        let dir = make_repo_on_branch("feature-123");
        let clc_dir = dir.join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: implementing\n").unwrap();

        let err = done(&dir, "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("must be terminal"),
            "expected terminal-phase error, got: {err}"
        );
    }
}

/// Resolve the active workflow for done ceremony.
fn resolve_done_workflow(project_dir: &Path, cfg: &config::Config) -> Workflow {
    let wf_name = phase::load_workflow_name(project_dir).unwrap_or(None);
    if let Some(name) = &wf_name {
        if let Some(def) = cfg.workflows.get(name) {
            if let Ok(wf) = Workflow::new(def) {
                return wf;
            }
        }
    }
    Workflow::default_tdd()
}
