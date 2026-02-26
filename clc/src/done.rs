use std::path::Path;
use std::process::Command;

use camino::Utf8Path;

use crate::error::Error;
use crate::git;
use crate::phase::{self, Phase};

pub fn done(project_dir: &Path, main_branch: &str) -> Result<(), Error> {
    // Must not be on main branch.
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if git_state.is_main {
        return Err(Error::NonBlocking(
            "cannot run 'done' on the main branch".into(),
        ));
    }

    // Phase must be green.
    let current_phase = phase::load(project_dir)?
        .ok_or_else(|| Error::NonBlocking("no phase set — nothing to finalize".into()))?;

    if current_phase == Phase::Done {
        return Err(Error::NonBlocking("already done".into()));
    }

    if current_phase != Phase::Green {
        return Err(Error::NonBlocking(format!(
            "phase must be 'green' to finalize, currently '{current_phase}'"
        )));
    }

    // Advance phase to done.
    phase::set(project_dir, "done", 1)?;

    // Stage the phase change.
    let mut paths_to_commit = vec![".clc/state"];

    // Close the tisket issue (branch name = issue ID).
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    if let Ok(repo) = tisket::Repo::open(utf8_dir) {
        let issue_id = &git_state.branch;
        match repo.close_issue(issue_id, Some("done")) {
            Ok(()) => {
                paths_to_commit.push(".tisket/");
            }
            Err(tisket::Error::IssueNotFound(_) | tisket::Error::IssueAlreadyClosed(_)) => {}
            Err(e) => {
                return Err(Error::NonBlocking(format!(
                    "failed to close tisket '{issue_id}': {e}"
                )));
            }
        }
    }

    // Commit the finalization changes.
    let mut add_args = vec!["add"];
    add_args.extend(paths_to_commit);

    let output = Command::new("git")
        .args(&add_args)
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to stage done changes: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::NonBlocking(format!(
            "failed to stage done changes: {stderr}"
        )));
    }

    let msg = format!("clc: finalize {}", git_state.branch);
    let output = Command::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to commit done changes: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::NonBlocking(format!(
            "failed to commit done changes: {stderr}"
        )));
    }

    Ok(())
}
