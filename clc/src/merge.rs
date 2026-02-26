use std::path::Path;
use std::process::Command;

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
    // If a worktree exists, read from its filesystem. Otherwise, read from the branch's tree.
    let worktree_dir = project_dir.join(".worktrees").join(id);
    let branch_phase = if worktree_dir.is_dir() {
        crate::phase::load(&worktree_dir)?
    } else {
        load_phase_from_tree(&branch_ref)?
    };

    match branch_phase {
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

    // Tisket must be closed.
    check_tisket_closed(&branch_ref, &worktree_dir, id)?;

    // Working tree must be clean (no modifications to tracked files).
    // Uses git-status because gix's status API requires the "status" feature which
    // brings in significant additional dependencies (dirwalk, blob-diff, index).
    if !working_tree_clean(project_dir)? {
        return Err(Error::NonBlocking(
            "working tree has uncommitted changes — commit or stash before merging".into(),
        ));
    }

    // Merge the branch (mutation — shell out to git).
    let output = Command::new("git")
        .args(["merge", id, "--ff"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to run git merge: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::NonBlocking(format!("git merge failed: {stderr}")));
    }

    // Clean up worktree if it exists (mutation).
    if worktree_dir.is_dir() {
        let _ = Command::new("git")
            .args(["worktree", "remove"])
            .arg(&worktree_dir)
            .current_dir(project_dir)
            .output();
    }

    // Clean up branch (mutation).
    let _ = Command::new("git")
        .args(["branch", "-d", id])
        .current_dir(project_dir)
        .output();

    Ok(())
}

/// Read the phase from a branch's committed `.clc/state` file via gix tree traversal.
fn load_phase_from_tree(branch_ref: &gix::Reference<'_>) -> Result<Option<Phase>, Error> {
    let mut ref_clone = branch_ref.clone();
    let tree = ref_clone
        .peel_to_tree()
        .map_err(|e| Error::NonBlocking(format!("failed to peel branch to tree: {e}")))?;

    let Ok(Some(entry)) = tree.lookup_entry_by_path(".clc/state") else {
        return Ok(None);
    };

    let object = entry
        .object()
        .map_err(|e| Error::NonBlocking(format!("failed to read .clc/state object: {e}")))?;

    let blob = object
        .try_into_blob()
        .map_err(|_| Error::NonBlocking(".clc/state is not a file".into()))?;

    let contents = std::str::from_utf8(&blob.data)
        .map_err(|_| Error::NonBlocking(".clc/state is not valid UTF-8".into()))?;

    let phase_str = contents
        .lines()
        .find_map(|line| line.strip_prefix("phase:").map(str::trim));

    match phase_str {
        Some(s) => Ok(Some(s.parse()?)),
        None => Ok(None),
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

/// Check if the working tree is clean (no modifications to tracked files).
/// Uses git-status because gix's status API requires the "status" feature
/// which brings significant additional dependencies.
fn working_tree_clean(project_dir: &Path) -> Result<bool, Error> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uno"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| Error::NonBlocking(format!("failed to run git status: {e}")))?;

    Ok(output.stdout.is_empty())
}
