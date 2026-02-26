use std::path::Path;

use crate::error::Error;
use crate::git;
use crate::phase::Phase;

pub fn merge(project_dir: &Path, id: &str, main_branch: &str) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "must be on the main branch to merge (currently on '{}')",
            git_state.branch
        )));
    }

    let repo = gix::discover(project_dir)
        .map_err(|_| Error::NonBlocking("not inside a git repository".into()))?;

    // Branch must exist.
    let ref_name = format!("refs/heads/{id}");
    let branch_ref = repo
        .find_reference(&ref_name)
        .map_err(|_| Error::NonBlocking(format!("branch '{id}' not found")))?;

    // Phase must be done on the feature branch.
    // If a worktree exists, read phase from its filesystem.
    // Otherwise, check for the finalization commit on the branch.
    // (.clc/state is never tracked by git — it's filesystem-only infrastructure state.)
    let worktree_dir = project_dir.join(".worktrees").join(id);
    if worktree_dir.is_dir() {
        match crate::phase::load(&worktree_dir)? {
            Some(Phase::Done) => {}
            Some(other) => {
                return Err(Error::NonBlocking(format!(
                    "branch '{id}' phase is '{other}', must be 'done' to merge"
                )));
            }
            None => {
                return Err(Error::NonBlocking(format!(
                    "branch '{id}' has no phase set, must be 'done' to merge"
                )));
            }
        }
    } else if !has_finalization_commit(&branch_ref, id)? {
        return Err(Error::NonBlocking(format!(
            "branch '{id}' has not been finalized — run 'clc done' first"
        )));
    }

    // Tisket must be closed.
    check_tisket_closed(&branch_ref, &worktree_dir, id)?;

    // Working tree must be clean (no modifications to tracked files).
    if !crate::gix_ops::working_tree_is_clean(project_dir)? {
        return Err(Error::NonBlocking(
            "working tree has uncommitted changes — commit or stash before merging".into(),
        ));
    }

    // Fast-forward merge via gix.
    crate::gix_ops::ff_merge(project_dir, id)?;

    // Clean up worktree if it exists.
    if worktree_dir.is_dir() {
        let _ = crate::gix_ops::remove_worktree(project_dir, &worktree_dir, id);
    }

    // Clean up branch.
    let _ = crate::gix_ops::delete_branch(project_dir, id);

    Ok(())
}

/// Check if the branch has a finalization commit (created by `clc done`).
/// Walks the branch's commit history looking for a "clc: finalize <id>" message.
fn has_finalization_commit(branch_ref: &gix::Reference<'_>, id: &str) -> Result<bool, Error> {
    let expected_prefix = format!("clc: finalize {id}");
    let mut ref_clone = branch_ref.clone();
    let commit = ref_clone
        .peel_to_commit()
        .map_err(|e| Error::NonBlocking(format!("failed to peel branch to commit: {e}")))?;

    // Check the tip commit first (most common case).
    let msg = commit.message_raw_sloppy();
    let msg_str = std::str::from_utf8(msg.as_ref()).unwrap_or("");
    if msg_str.starts_with(&expected_prefix) {
        return Ok(true);
    }

    // Walk up to a few parents in case there were commits after finalization.
    let mut current = commit.id().detach();
    for _ in 0..10 {
        let obj = branch_ref
            .repo
            .find_object(current)
            .map_err(|e| Error::NonBlocking(format!("failed to find commit: {e}")))?
            .into_commit();

        let parent_ids: Vec<gix::ObjectId> = obj.parent_ids().map(gix::Id::detach).collect();
        let Some(parent_id) = parent_ids.first() else {
            break;
        };

        let parent = branch_ref
            .repo
            .find_object(*parent_id)
            .map_err(|e| Error::NonBlocking(format!("failed to find parent commit: {e}")))?
            .into_commit();

        let parent_msg = parent.message_raw_sloppy();
        let parent_msg_str = std::str::from_utf8(parent_msg.as_ref()).unwrap_or("");
        if parent_msg_str.starts_with(&expected_prefix) {
            return Ok(true);
        }

        current = *parent_id;
    }

    Ok(false)
}

/// Check that the tisket for `id` is closed. Uses worktree filesystem if available,
/// otherwise reads from the branch's tree via gix.
fn check_tisket_closed(
    branch_ref: &gix::Reference<'_>,
    worktree_dir: &Path,
    id: &str,
) -> Result<(), Error> {
    if worktree_dir.is_dir() {
        let utf8_dir = camino::Utf8Path::new(
            worktree_dir
                .to_str()
                .ok_or_else(|| Error::NonBlocking("non-UTF8 path".into()))?,
        );
        if let Ok(repo) = tisket::Repo::open(utf8_dir)
            && let Ok(issue) = repo.find_issue(id)
            && !issue.closed
        {
            return Err(Error::NonBlocking(format!(
                "tisket '{id}' is not closed (status: {})",
                issue.frontmatter.status
            )));
        }
        return Ok(());
    }

    // No worktree — check the branch's tree via gix.
    // Tisket marks issues as closed by moving them to `.closed/` subdirectory.
    let mut ref_clone = branch_ref.clone();
    let Ok(tree) = ref_clone.peel_to_tree() else {
        return Ok(()); // Can't read tree, skip tisket check.
    };

    // Check if the closed marker exists.
    let closed_path = format!(".tisket/v0.1.0/.closed/{id}.md");
    let has_closed = matches!(tree.lookup_entry_by_path(&*closed_path), Ok(Some(_)));

    if !has_closed {
        // Check if the issue exists in the open location.
        let open_path = format!(".tisket/v0.1.0/{id}.md");
        let has_open = matches!(tree.lookup_entry_by_path(&*open_path), Ok(Some(_)));

        if has_open {
            return Err(Error::NonBlocking(format!(
                "tisket '{id}' is not closed on branch '{id}'"
            )));
        }
    }

    Ok(())
}
