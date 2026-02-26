use std::path::Path;
use std::process::Command;

use crate::error::Error;
use crate::git;

const ADMIN_BRANCH: &str = "clc-admin";

pub fn admin(project_dir: &Path, main_branch: &str) -> Result<(), Error> {
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "must be on the main branch to create admin worktree (currently on '{}')",
            git_state.branch
        )));
    }

    let worktree_dir = project_dir.join(".worktrees").join(ADMIN_BRANCH);

    if worktree_dir.exists() {
        // Already exists — idempotent success.
        return Ok(());
    }

    // Create git worktree.
    let output = Command::new("git")
        .args(["worktree", "add"])
        .arg(&worktree_dir)
        .args(["-b", ADMIN_BRANCH])
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::NonBlocking(format!(
            "git worktree add failed: {stderr}"
        )));
    }

    // Initialize clc in the admin worktree (no phase set — admin has no phase).
    crate::init::init(&worktree_dir, false, true)?;

    Ok(())
}
