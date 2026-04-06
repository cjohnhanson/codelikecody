use std::path::Path;

use crate::error::Error;
use crate::git;
use crate::workflow::Workflow;

pub fn merge(project_dir: &Path, id: &str, main_branch: &str, admin_branch: &str) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch, admin_branch)
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
        let cfg = crate::config::load(&worktree_dir).unwrap_or_default();
        let wf_name = crate::phase::load_workflow_name(&worktree_dir).unwrap_or(None);
        let workflow = wf_name
            .as_ref()
            .and_then(|name| cfg.workflows.get(name))
            .and_then(|def| Workflow::new(def).ok())
            .unwrap_or_else(Workflow::default_tdd);

        match crate::phase::load_name(&worktree_dir)? {
            Some(ref name) if workflow.is_terminal(name) => {}
            Some(name) => {
                return Err(Error::NonBlocking(format!(
                    "branch '{id}' phase is '{name}', must be terminal to merge"
                )));
            }
            None => {
                return Err(Error::NonBlocking(format!(
                    "branch '{id}' has no phase set, must be terminal to merge"
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

#[cfg(test)]
mod tests {
    use super::*;
    use gix::prelude::Write as _;

    /// Create a temporary git repo on main branch with an initial commit.
    fn make_main_repo() -> std::path::PathBuf {
        #[allow(deprecated)]
        let dir = tempfile::tempdir().unwrap().into_path();

        gix::init(&dir).unwrap();

        let config_path = dir.join(".git").join("config");
        let mut config = std::fs::read_to_string(&config_path).unwrap_or_default();
        config.push_str("[user]\n\tname = test\n\temail = test@test\n");
        std::fs::write(&config_path, config).unwrap();

        let repo = gix::open(&dir).unwrap();
        let empty_tree = repo.write(&gix::objs::Tree::empty()).unwrap();
        repo.commit("HEAD", "initial", empty_tree, gix::commit::NO_PARENT_IDS)
            .unwrap();

        dir
    }

    /// Create a repo on a named branch (not main).
    fn make_repo_on_branch(branch: &str) -> std::path::PathBuf {
        let dir = make_main_repo();
        let repo = gix::open(&dir).unwrap();
        let head_id = repo.head_id().unwrap().detach();
        let ref_name = format!("refs/heads/{branch}");
        repo.reference(
            ref_name.clone(),
            head_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "create branch",
        )
        .unwrap();
        let head_ref_path = dir.join(".git").join("HEAD");
        std::fs::write(&head_ref_path, format!("ref: {ref_name}\n")).unwrap();
        dir
    }

    #[test]
    fn merge_rejects_when_not_on_main() {
        let dir = make_repo_on_branch("feature-x");
        let err = merge(&dir, "feature-x", "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("must be on the main branch"),
            "expected main-branch error, got: {err}"
        );
    }

    #[test]
    fn merge_rejects_nonexistent_branch() {
        let dir = make_main_repo();
        let err = merge(&dir, "nonexistent", "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("not found"),
            "expected branch-not-found error, got: {err}"
        );
    }

    #[test]
    fn merge_rejects_unfinalized_branch() {
        let dir = make_main_repo();

        // Create a feature branch with a commit but no finalization.
        let repo = gix::open(&dir).unwrap();
        let head_id = repo.head_id().unwrap().detach();
        repo.reference(
            "refs/heads/feat-abc",
            head_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "create branch",
        )
        .unwrap();

        let err = merge(&dir, "feat-abc", "main", "admin").unwrap_err();
        assert!(
            err.to_string().contains("not been finalized"),
            "expected not-finalized error, got: {err}"
        );
    }
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
