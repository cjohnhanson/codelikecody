use std::path::Path;

use gix::refs::transaction::PreviousValue;

use crate::error::Error;

const INTEGRATE_PREFIX: &str = "integrate/";

/// Create a new integration branch at the current main HEAD.
/// Must be called from the main branch.
pub fn create(project_dir: &Path, name: &str, main_branch: &str) -> Result<(), Error> {
    let repo = discover(project_dir)?;
    let branch = current_branch(&repo)?;

    if branch != main_branch {
        return Err(Error::NonBlocking(format!(
            "must be on '{main_branch}' to create an integration branch (currently on '{branch}')"
        )));
    }

    let branch_name = format!("{INTEGRATE_PREFIX}{name}");
    let head_id = repo
        .head_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD: {e}")))?
        .detach();

    // Create the branch ref.
    let ref_name = format!("refs/heads/{branch_name}");
    repo.reference(
        ref_name.as_str(),
        head_id,
        PreviousValue::MustNotExist,
        "integrate: create integration branch",
    )
    .map_err(|e| Error::NonBlocking(format!("failed to create branch '{branch_name}': {e}")))?;

    // Switch HEAD to the new branch.
    set_head_to_branch(&repo, &branch_name)?;

    Ok(())
}

/// Merge a worker branch into the current integration branch.
/// Must be called from an integrate/* branch.
pub fn merge(project_dir: &Path, branch_name: &str) -> Result<(), Error> {
    let repo = discover(project_dir)?;
    let current = current_branch(&repo)?;

    if !current.starts_with(INTEGRATE_PREFIX) {
        return Err(Error::NonBlocking(format!(
            "must be on an integration branch to merge (currently on '{current}')"
        )));
    }

    // Resolve the worker branch.
    let their_ref_name = format!("refs/heads/{branch_name}");
    let their_ref = repo
        .find_reference(&their_ref_name)
        .map_err(|_| Error::NonBlocking(format!("branch '{branch_name}' not found")))?;
    let their_id = their_ref.id().detach();

    let our_id = repo
        .head_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD: {e}")))?
        .detach();

    // Check if this branch is already merged (their commit is ancestor of ours).
    if is_ancestor(&repo, their_id, our_id)? {
        return Err(Error::NonBlocking(format!(
            "branch '{branch_name}' is already merged into the integration branch"
        )));
    }

    // Find the merge base (common ancestor).
    let base_id = find_merge_base(&repo, our_id, their_id)?;

    // Get tree IDs for the three-way merge.
    let base_tree_id = commit_tree_id(&repo, base_id)?;
    let our_tree_id = commit_tree_id(&repo, our_id)?;
    let their_tree_id = commit_tree_id(&repo, their_id)?;

    // Perform the three-way merge.
    let options = repo
        .tree_merge_options()
        .map_err(|e| Error::NonBlocking(format!("failed to get merge options: {e}")))?;

    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: Some(b"base".into()),
        current: Some(b"ours".into()),
        other: Some(b"theirs".into()),
    };

    let mut outcome = repo
        .merge_trees(base_tree_id, our_tree_id, their_tree_id, labels, options)
        .map_err(|e| Error::NonBlocking(format!("merge failed: {e}")))?;

    if outcome.has_unresolved_conflicts(gix::merge::tree::TreatAsUnresolved::default()) {
        return Err(Error::NonBlocking(format!(
            "merge conflict: cannot merge '{branch_name}' — conflicts must be resolved manually"
        )));
    }

    // Write the merged tree.
    let merged_tree_id = outcome
        .tree
        .write()
        .map_err(|e| Error::NonBlocking(format!("failed to write merged tree: {e}")))?
        .detach();

    // Create merge commit with two parents.
    let message = format!("integrate: merge {branch_name}");
    let _merge_commit = repo
        .commit("HEAD", &message, merged_tree_id, [our_id, their_id])
        .map_err(|e| Error::NonBlocking(format!("failed to create merge commit: {e}")))?;

    // Update index and working tree.
    update_worktree(&repo, project_dir, &merged_tree_id)?;

    Ok(())
}

/// Squash-merge the integration branch onto main and clean up.
/// Must be called from an integrate/* branch.
pub fn land(project_dir: &Path, main_branch: &str) -> Result<(), Error> {
    let repo = discover(project_dir)?;
    let current = current_branch(&repo)?;

    if !current.starts_with(INTEGRATE_PREFIX) {
        return Err(Error::NonBlocking(format!(
            "must be on an integration branch to land (currently on '{current}')"
        )));
    }

    let integration_id = repo
        .head_id()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD: {e}")))?
        .detach();

    // Resolve main branch.
    let main_ref_name = format!("refs/heads/{main_branch}");
    let main_ref = repo
        .find_reference(&main_ref_name)
        .map_err(|_| Error::NonBlocking(format!("branch '{main_branch}' not found")))?;
    let main_id = main_ref.id().detach();

    // Verify the integration branch is ahead of main.
    if integration_id == main_id {
        return Err(Error::NonBlocking(
            "integration branch has no changes to land (same as main)".to_string(),
        ));
    }

    if !is_ancestor(&repo, main_id, integration_id)? {
        return Err(Error::NonBlocking(
            "integration branch is not ahead of main — rebase may be needed".to_string(),
        ));
    }

    // Collect the worker branches that were merged (parents of merge commits
    // on the integration branch that aren't on main).
    let merged_branches = collect_merged_branches(&repo, main_id, integration_id)?;

    // Get the integration branch's tree — this is the final merged state.
    let integration_tree_id = commit_tree_id(&repo, integration_id)?;

    // Build the squash commit message.
    let message = build_squash_message(&current, &merged_branches);

    // Switch to main.
    set_head_to_branch(&repo, main_branch)?;

    // Create the squash commit on main: single parent (main), tree from integration.
    let _squash_commit = repo
        .commit("HEAD", &message, integration_tree_id, [main_id])
        .map_err(|e| Error::NonBlocking(format!("failed to create squash commit: {e}")))?;

    // Update index and working tree to match the new main HEAD.
    update_worktree(&repo, project_dir, &integration_tree_id)?;

    // Clean up: delete integration branch and merged worker branches.
    let integration_ref = format!("refs/heads/{current}");
    if let Ok(r) = repo.find_reference(&integration_ref) {
        let _ = r.delete();
    }

    for branch in &merged_branches {
        let ref_name = format!("refs/heads/{branch}");
        if let Ok(r) = repo.find_reference(&ref_name) {
            let _ = r.delete();
        }
    }

    Ok(())
}

// --- Internal helpers ---

fn discover(project_dir: &Path) -> Result<gix::Repository, Error> {
    gix::discover(project_dir).map_err(|_| Error::NonBlocking("not inside a git repository".into()))
}

fn current_branch(repo: &gix::Repository) -> Result<String, Error> {
    let head = repo
        .head_ref()
        .map_err(|e| Error::NonBlocking(format!("failed to get HEAD ref: {e}")))?
        .ok_or_else(|| Error::NonBlocking("HEAD is detached".into()))?;

    let name = head.name().shorten().to_string();
    Ok(name)
}

fn set_head_to_branch(repo: &gix::Repository, branch_name: &str) -> Result<(), Error> {
    let ref_name = format!("refs/heads/{branch_name}");
    let head_path = repo.path().join("HEAD");
    std::fs::write(&head_path, format!("ref: {ref_name}\n"))
        .map_err(|e| Error::NonBlocking(format!("failed to switch to '{branch_name}': {e}")))?;
    Ok(())
}

fn commit_tree_id(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
) -> Result<gix::ObjectId, Error> {
    let commit = repo
        .find_object(commit_id)
        .map_err(|e| Error::NonBlocking(format!("failed to find commit {commit_id}: {e}")))?
        .into_commit();
    commit
        .tree_id()
        .map(gix::Id::detach)
        .map_err(|e| Error::NonBlocking(format!("failed to get tree from commit: {e}")))
}

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
        for parent_id in commit.parent_ids().map(gix::Id::detach) {
            if seen.insert(parent_id) {
                queue.push_back(parent_id);
            }
        }
    }
    Ok(false)
}

fn find_merge_base(
    repo: &gix::Repository,
    a: gix::ObjectId,
    b: gix::ObjectId,
) -> Result<gix::ObjectId, Error> {
    // Walk ancestors of both commits, find the first common one.
    let mut ancestors_a = std::collections::HashSet::new();
    let mut queue_a = std::collections::VecDeque::new();
    queue_a.push_back(a);
    ancestors_a.insert(a);

    while let Some(current) = queue_a.pop_front() {
        let commit = repo
            .find_object(current)
            .map_err(|e| Error::NonBlocking(format!("failed to find commit: {e}")))?
            .into_commit();
        for pid in commit.parent_ids().map(gix::Id::detach) {
            if ancestors_a.insert(pid) {
                queue_a.push_back(pid);
            }
        }
    }

    let mut queue_b = std::collections::VecDeque::new();
    let mut seen_b = std::collections::HashSet::new();
    queue_b.push_back(b);
    seen_b.insert(b);

    while let Some(current) = queue_b.pop_front() {
        if ancestors_a.contains(&current) {
            return Ok(current);
        }
        let commit = repo
            .find_object(current)
            .map_err(|e| Error::NonBlocking(format!("failed to find commit: {e}")))?
            .into_commit();
        for pid in commit.parent_ids().map(gix::Id::detach) {
            if seen_b.insert(pid) {
                queue_b.push_back(pid);
            }
        }
    }

    Err(Error::NonBlocking(
        "no common ancestor found between commits".to_string(),
    ))
}

fn update_worktree(
    repo: &gix::Repository,
    work_dir: &Path,
    tree_id: &gix::ObjectId,
) -> Result<(), Error> {
    use std::sync::atomic::AtomicBool;

    let mut index = repo
        .index_from_tree(tree_id)
        .map_err(|e| Error::NonBlocking(format!("failed to create index from tree: {e}")))?;

    let mut opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
        .map_err(|e| Error::NonBlocking(format!("failed to get checkout options: {e}")))?;
    opts.overwrite_existing = true;

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

    let index_path = repo.path().join("index");
    index
        .write(gix::index::write::Options::default())
        .map_err(|e| Error::NonBlocking(format!("failed to write index: {e}")))?;
    index.set_path(index_path);

    Ok(())
}

/// Collect branch names that were merged into the integration branch
/// (second parents of merge commits between main and integration HEAD).
fn collect_merged_branches(
    repo: &gix::Repository,
    main_id: gix::ObjectId,
    integration_id: gix::ObjectId,
) -> Result<Vec<String>, Error> {
    let mut branches = Vec::new();
    let mut current = integration_id;

    // Walk backwards from integration HEAD to main, collecting second parents of merge commits.
    loop {
        if current == main_id {
            break;
        }

        let commit = repo
            .find_object(current)
            .map_err(|e| Error::NonBlocking(format!("failed to find commit: {e}")))?
            .into_commit();

        let parent_ids: Vec<gix::ObjectId> = commit.parent_ids().map(gix::Id::detach).collect();

        // If this is a merge commit (2+ parents), the second parent is the merged branch.
        if parent_ids.len() >= 2 {
            // Try to find a branch name pointing to the second parent.
            if let Some(name) = find_branch_for_commit(repo, parent_ids[1]) {
                branches.push(name);
            }
        }

        if parent_ids.is_empty() {
            break;
        }
        current = parent_ids[0]; // Follow first parent (integration branch history).
    }

    branches.reverse(); // Oldest first.
    Ok(branches)
}

/// Find a branch name whose tip is the given commit.
fn find_branch_for_commit(repo: &gix::Repository, commit_id: gix::ObjectId) -> Option<String> {
    let refs = repo.references().ok()?;
    let local = refs.local_branches().ok()?;
    for reference in local.flatten() {
        if reference.id().detach() == commit_id {
            return Some(reference.name().shorten().to_string());
        }
    }
    None
}

fn build_squash_message(integration_branch: &str, merged_branches: &[String]) -> String {
    use std::fmt::Write as _;
    let mut msg = format!("integrate: land {integration_branch}\n\n");
    if merged_branches.is_empty() {
        msg.push_str("No worker branches identified.\n");
    } else {
        msg.push_str("Merged branches:\n");
        for branch in merged_branches {
            let _ = writeln!(msg, "- {branch}");
        }
    }
    msg
}
