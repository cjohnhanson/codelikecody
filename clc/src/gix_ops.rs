use std::path::Path;
use std::sync::atomic::AtomicBool;

use gix::object::tree::EntryKind;
use gix::refs::transaction::PreviousValue;

use crate::error::Error;

/// Commit specific filesystem paths by building a new tree from the current HEAD tree
/// with the given paths updated, then creating a commit.
///
/// Paths can be files (e.g. ".clc/state") or directories (e.g. ".tisket/").
/// Directory paths are walked recursively and all files inside are added.
pub fn commit_paths(project_dir: &Path, message: &str, paths: &[&str]) -> Result<(), Error> {
    let repo = open(project_dir)?;
    let head_commit = repo
        .head_commit()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD commit: {e}")))?;

    let head_tree = head_commit
        .tree()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD tree: {e}")))?;

    let mut editor = head_tree
        .edit()
        .map_err(|e| Error::NonBlocking(format!("failed to create tree editor: {e}")))?;

    for &path in paths {
        let fs_path = project_dir.join(path);

        if fs_path.is_dir() {
            add_directory_to_editor(&repo, &mut editor, project_dir, path)?;
        } else if fs_path.is_file() {
            let contents = std::fs::read(&fs_path).map_err(|e| {
                Error::NonBlocking(format!("failed to read {}: {e}", fs_path.display()))
            })?;
            let blob_id = repo
                .write_blob(&contents)
                .map_err(|e| Error::NonBlocking(format!("failed to write blob for {path}: {e}")))?;
            editor
                .upsert(path, EntryKind::Blob, blob_id)
                .map_err(|e| Error::NonBlocking(format!("failed to upsert {path}: {e}")))?;
        }
    }

    let tree_id = editor
        .write()
        .map_err(|e| Error::NonBlocking(format!("failed to write tree: {e}")))?;

    repo.commit("HEAD", message, tree_id, [head_commit.id])
        .map_err(|e| Error::NonBlocking(format!("failed to create commit: {e}")))?;

    // Update the index to match the new tree so the working tree appears clean.
    let mut index = repo
        .index_from_tree(&tree_id)
        .map_err(|e| Error::NonBlocking(format!("failed to rebuild index: {e}")))?;

    let index_path = repo.path().join("index");
    index
        .write(gix::index::write::Options::default())
        .map_err(|e| Error::NonBlocking(format!("failed to write index: {e}")))?;
    index.set_path(index_path);

    Ok(())
}

/// Fast-forward merge: advance HEAD to the target branch's commit.
/// The working tree must be clean before calling this.
pub fn ff_merge(project_dir: &Path, branch_name: &str) -> Result<(), Error> {
    let repo = open(project_dir)?;

    let ref_name = format!("refs/heads/{branch_name}");
    let branch_ref = repo
        .find_reference(&ref_name)
        .map_err(|_| Error::NonBlocking(format!("branch '{branch_name}' not found")))?;

    let target_id = branch_ref.id().detach();

    let head_id = repo
        .head_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD: {e}")))?
        .detach();

    // Verify this is actually a fast-forward (target is descendant of HEAD).
    // Walk ancestors of target to find head_id.
    if target_id != head_id && !is_ancestor(&repo, head_id, target_id)? {
        return Err(Error::NonBlocking(format!(
            "cannot fast-forward: '{branch_name}' is not a descendant of HEAD"
        )));
    }

    // Update HEAD's target ref to point to the new commit.
    let mut head_ref = repo
        .head_ref()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD ref: {e}")))?
        .ok_or_else(|| Error::NonBlocking("HEAD is detached".into()))?;

    head_ref
        .set_target_id(target_id, format!("merge {branch_name}: fast-forward"))
        .map_err(|e| Error::NonBlocking(format!("failed to update HEAD: {e}")))?;

    // Update index and working tree to match the new commit's tree.
    let target_commit = repo
        .find_object(target_id)
        .map_err(|e| Error::NonBlocking(format!("failed to find target commit: {e}")))?
        .into_commit();

    let target_tree_id = target_commit
        .tree_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get target tree: {e}")))?
        .detach();

    checkout_tree(&repo, project_dir, &target_tree_id)?;

    Ok(())
}

/// Create a new git worktree at `worktree_path` with a new branch `branch_name`.
/// The branch is created pointing at the current HEAD commit.
pub fn create_worktree(
    project_dir: &Path,
    worktree_path: &Path,
    branch_name: &str,
) -> Result<(), Error> {
    let repo = open(project_dir)?;

    let head_id = repo
        .head_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD: {e}")))?
        .detach();

    // Create the branch ref pointing to HEAD.
    let ref_name = format!("refs/heads/{branch_name}");
    repo.reference(
        ref_name.as_str(),
        head_id,
        PreviousValue::MustNotExist,
        "branch: Created from HEAD for worktree",
    )
    .map_err(|e| Error::NonBlocking(format!("failed to create branch '{branch_name}': {e}")))?;

    // Determine the main git directory.
    // For a regular repo, this is `.git/`. For a linked worktree, use common_dir.
    let git_dir = repo.common_dir().to_path_buf();

    let worktree_name = branch_name;
    let wt_git_dir = git_dir.join("worktrees").join(worktree_name);

    // Create .git/worktrees/<name>/ directory.
    std::fs::create_dir_all(&wt_git_dir).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to create worktree git dir {}: {e}",
            wt_git_dir.display()
        ))
    })?;

    // Write .git/worktrees/<name>/commondir
    std::fs::write(wt_git_dir.join("commondir"), "../..\n")
        .map_err(|e| Error::NonBlocking(format!("failed to write commondir: {e}")))?;

    // Write .git/worktrees/<name>/HEAD
    std::fs::write(wt_git_dir.join("HEAD"), format!("ref: {ref_name}\n"))
        .map_err(|e| Error::NonBlocking(format!("failed to write worktree HEAD: {e}")))?;

    // Write .git/worktrees/<name>/gitdir (absolute path to worktree's .git file)
    let abs_worktree = worktree_path
        .canonicalize()
        .or_else(|_| {
            // Path may not exist yet; create it first then canonicalize.
            std::fs::create_dir_all(worktree_path)?;
            worktree_path.canonicalize()
        })
        .map_err(|e| {
            Error::NonBlocking(format!(
                "failed to resolve worktree path {}: {e}",
                worktree_path.display()
            ))
        })?;

    std::fs::write(
        wt_git_dir.join("gitdir"),
        format!("{}\n", abs_worktree.join(".git").display()),
    )
    .map_err(|e| Error::NonBlocking(format!("failed to write gitdir: {e}")))?;

    // Create the worktree directory if it doesn't exist.
    std::fs::create_dir_all(worktree_path).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to create worktree dir {}: {e}",
            worktree_path.display()
        ))
    })?;

    // Write <worktree>/.git file (not directory) pointing back.
    std::fs::write(
        worktree_path.join(".git"),
        format!("gitdir: {}\n", wt_git_dir.display()),
    )
    .map_err(|e| Error::NonBlocking(format!("failed to write .git file: {e}")))?;

    // Checkout the tree into the worktree directory.
    let head_tree_id = repo
        .head_tree_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD tree: {e}")))?
        .detach();

    // Open the worktree as its own repository for checkout.
    let wt_repo = gix::open(worktree_path)
        .map_err(|e| Error::NonBlocking(format!("failed to open worktree repo: {e}")))?;

    checkout_tree(&wt_repo, worktree_path, &head_tree_id)?;

    Ok(())
}

/// Remove a worktree directory and its git metadata.
pub fn remove_worktree(project_dir: &Path, worktree_path: &Path, name: &str) -> Result<(), Error> {
    let repo = open(project_dir)?;
    let git_dir = repo.common_dir().to_path_buf();

    // Remove the worktree directory.
    if worktree_path.is_dir() {
        std::fs::remove_dir_all(worktree_path).map_err(|e| {
            Error::NonBlocking(format!(
                "failed to remove worktree {}: {e}",
                worktree_path.display()
            ))
        })?;
    }

    // Remove .git/worktrees/<name>/.
    let wt_git_dir = git_dir.join("worktrees").join(name);
    if wt_git_dir.is_dir() {
        std::fs::remove_dir_all(&wt_git_dir).map_err(|e| {
            Error::NonBlocking(format!(
                "failed to remove worktree git dir {}: {e}",
                wt_git_dir.display()
            ))
        })?;
    }

    Ok(())
}

/// Delete a branch reference.
pub fn delete_branch(project_dir: &Path, branch_name: &str) -> Result<(), Error> {
    let repo = open(project_dir)?;
    let ref_name = format!("refs/heads/{branch_name}");

    let reference = repo
        .find_reference(&ref_name)
        .map_err(|_| Error::NonBlocking(format!("branch '{branch_name}' not found")))?;

    reference
        .delete()
        .map_err(|e| Error::NonBlocking(format!("failed to delete branch '{branch_name}': {e}")))?;

    Ok(())
}

/// Check if the working tree is clean (no modifications to tracked files).
/// Untracked files are ignored.
pub fn working_tree_is_clean(project_dir: &Path) -> Result<bool, Error> {
    let repo = open(project_dir)?;

    let is_dirty = repo
        .is_dirty()
        .map_err(|e| Error::NonBlocking(format!("failed to check working tree status: {e}")))?;

    Ok(!is_dirty)
}

// --- Internal helpers ---

fn open(project_dir: &Path) -> Result<gix::Repository, Error> {
    gix::discover(project_dir).map_err(|_| Error::NonBlocking("not inside a git repository".into()))
}

/// Walk a directory recursively and add all files to the tree editor.
fn add_directory_to_editor(
    repo: &gix::Repository,
    editor: &mut gix::object::tree::Editor<'_>,
    project_dir: &Path,
    dir_rel_path: &str,
) -> Result<(), Error> {
    let fs_dir = project_dir.join(dir_rel_path);
    let mut stack = vec![fs_dir];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| {
            Error::NonBlocking(format!("failed to read directory {}: {e}", dir.display()))
        })?;

        for entry in entries {
            let entry =
                entry.map_err(|e| Error::NonBlocking(format!("failed to read dir entry: {e}")))?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let rel_path = path
                    .strip_prefix(project_dir)
                    .map_err(|_| {
                        Error::NonBlocking(format!("path {} outside project dir", path.display()))
                    })?
                    .to_string_lossy();

                let contents = std::fs::read(&path).map_err(|e| {
                    Error::NonBlocking(format!("failed to read {}: {e}", path.display()))
                })?;

                let blob_id = repo.write_blob(&contents).map_err(|e| {
                    Error::NonBlocking(format!("failed to write blob for {rel_path}: {e}"))
                })?;

                editor
                    .upsert(rel_path.as_ref(), EntryKind::Blob, blob_id)
                    .map_err(|e| Error::NonBlocking(format!("failed to upsert {rel_path}: {e}")))?;
            }
        }
    }

    Ok(())
}

/// Check if `ancestor` is an ancestor of `descendant` by walking the commit graph.
fn is_ancestor(
    repo: &gix::Repository,
    ancestor: gix::ObjectId,
    descendant: gix::ObjectId,
) -> Result<bool, Error> {
    let mut queue = std::collections::VecDeque::new();
    let mut seen = std::collections::HashSet::new();

    queue.push_back(descendant);
    seen.insert(descendant);

    while let Some(current) = queue.pop_front() {
        if current == ancestor {
            return Ok(true);
        }

        let commit = repo
            .find_object(current)
            .map_err(|e| Error::NonBlocking(format!("failed to find commit {current}: {e}")))?
            .into_commit();

        let parent_ids: Vec<gix::ObjectId> = commit.parent_ids().map(gix::Id::detach).collect();

        for parent_id in parent_ids {
            if seen.insert(parent_id) {
                queue.push_back(parent_id);
            }
        }
    }

    Ok(false)
}

/// Checkout a tree into a working directory by writing an index and running checkout.
fn checkout_tree(
    repo: &gix::Repository,
    work_dir: &Path,
    tree_id: &gix::ObjectId,
) -> Result<(), Error> {
    let mut index = repo
        .index_from_tree(tree_id)
        .map_err(|e| Error::NonBlocking(format!("failed to create index from tree: {e}")))?;

    let mut opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
        .map_err(|e| Error::NonBlocking(format!("failed to get checkout options: {e}")))?;
    opts.overwrite_existing = true;
    opts.destination_is_initially_empty = true;

    let files = gix::progress::Discard;
    let bytes = gix::progress::Discard;

    gix::worktree::state::checkout(
        &mut index,
        work_dir,
        repo.objects.clone().into_arc().map_err(|e| {
            Error::NonBlocking(format!("failed to get thread-safe object store: {e}"))
        })?,
        &files,
        &bytes,
        &AtomicBool::new(false),
        opts,
    )
    .map_err(|e| Error::NonBlocking(format!("failed to checkout tree: {e}")))?;

    // Write the index file.
    let index_path = repo.path().join("index");
    index
        .write(gix::index::write::Options::default())
        .map_err(|e| Error::NonBlocking(format!("failed to write index: {e}")))?;
    index.set_path(index_path);

    Ok(())
}
