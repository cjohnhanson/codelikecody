use std::path::Path;

use camino::Utf8Path;

use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;
use crate::phase::{self, Phase};

pub fn done(project_dir: &Path, main_branch: &str, admin_branch: &str) -> Result<(), Error> {
    // Must not be on main branch.
    let git_state = git::detect(project_dir, main_branch, admin_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if git_state.is_main {
        return Err(Error::NonBlocking(
            "cannot run 'done' on the main branch".into(),
        ));
    }

    // Phase must be done (coordinator advances to done before calling `clc done`).
    let current_phase = phase::load(project_dir)?
        .ok_or_else(|| Error::NonBlocking("no phase set — nothing to finalize".into()))?;

    if current_phase != Phase::Done {
        return Err(Error::NonBlocking(format!(
            "phase must be 'done' to finalize, currently '{current_phase}'"
        )));
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
