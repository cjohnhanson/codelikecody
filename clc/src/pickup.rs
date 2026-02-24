use std::path::Path;
use std::process::Command;

use camino::Utf8Path;

use crate::error::Error;
use crate::git;

pub fn pickup(project_dir: &Path, id: &str, main_branch: &str) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "must be on the main branch to pick up a tisket (currently on '{}')",
            git_state.branch
        )));
    }

    // Find the tisket issue and verify status.
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo = tisket::Repo::open(utf8_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to open tisket repo: {e}")))?;

    let issue = repo
        .find_issue(id)
        .map_err(|e| Error::NonBlocking(format!("tisket issue '{id}' not found: {e}")))?;

    if issue.frontmatter.status != "todo" {
        return Err(Error::NonBlocking(format!(
            "tisket '{id}' is in '{}' status, must be 'todo' to pick up",
            issue.frontmatter.status
        )));
    }

    // Check depends_on are all resolved.
    for dep_id in &issue.frontmatter.depends_on {
        match repo.find_issue(dep_id) {
            Ok(dep) if !dep.closed => {
                return Err(Error::NonBlocking(format!(
                    "tisket '{id}' depends on '{dep_id}' which is not closed"
                )));
            }
            Err(_) => {
                return Err(Error::NonBlocking(format!(
                    "tisket '{id}' depends on '{dep_id}' which was not found"
                )));
            }
            _ => {} // closed, ok
        }
    }

    // Create git worktree.
    let worktree_dir = project_dir.join(".worktrees").join(id);
    let output = Command::new("git")
        .args(["worktree", "add"])
        .arg(&worktree_dir)
        .args(["-b", id])
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::NonBlocking(format!(
            "git worktree add failed: {stderr}"
        )));
    }

    // Set tisket status to in_progress.
    repo.edit_issue(id, Some("in_progress"))
        .map_err(|e| Error::NonBlocking(format!("failed to update tisket status: {e}")))?;

    // Initialize clc in the worktree.
    crate::init::init(&worktree_dir, false)?;

    // Set initial phase.
    crate::phase::set(&worktree_dir, "tests-unwritten", 1)?;

    Ok(())
}
