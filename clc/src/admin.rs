use std::path::Path;

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

    // Create git worktree via gix.
    crate::gix_ops::create_worktree(project_dir, &worktree_dir, ADMIN_BRANCH)?;

    // Initialize clc in the admin worktree (no phase set — admin has no phase).
    crate::init::init(&worktree_dir, false, true)?;

    Ok(())
}
