use std::path::Path;

use camino::Utf8Path;

use crate::error::Error;
use crate::git;

pub fn pickup(
    project_dir: &Path,
    id: &str,
    main_branch: &str,
    admin_branch: &str,
    coordinator_id: Option<&str>,
) -> Result<(), Error> {
    // Must be on main branch.
    let git_state = git::detect(project_dir, main_branch, admin_branch)
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

    if !issue.frontmatter.status.is_pickable() {
        return Err(Error::NonBlocking(format!(
            "tisket '{id}' is in '{}' status, not pickable",
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

    // Set tisket status to in_progress (and assignee if coordinator) and commit
    // on trunk. This must happen BEFORE creating the worktree so the branch
    // forks from a HEAD that includes the status change.
    repo.edit_issue(
        id,
        tisket::EditIssueOptions {
            status: Some("in_progress"),
            assignee: coordinator_id,
            ..Default::default()
        },
    )
    .map_err(|e| Error::NonBlocking(format!("failed to update tisket status: {e}")))?;

    repo.ensure_scratch_notes(id)
        .map_err(|e| Error::NonBlocking(format!("failed to add scratch notes section: {e}")))?;

    let msg = format!("clc: pickup {id}");
    crate::gix_ops::commit_paths(project_dir, &msg, &[".tisket/"])?;

    // Create git worktree from the new HEAD (which includes the status change).
    let worktree_dir = project_dir.join(".worktrees").join(id);
    crate::gix_ops::create_worktree(project_dir, &worktree_dir, id)?;

    // Initialize clc in the worktree.
    crate::init::init(&worktree_dir, false, true)?;

    // Resolve initial phase from workflow policy.
    // If no workflows are configured, fall back to the standard TDD starting phase.
    let config = crate::config::load(project_dir).unwrap_or_default();
    let initial_phase = if config.workflows.is_empty() {
        "tests-unwritten"
    } else {
        let workflow = config.resolve_workflow(&issue.frontmatter.labels, &issue.project);
        workflow.phases.first().map(|s| s.as_str()).unwrap_or("done")
    };

    crate::phase::init_phase(&worktree_dir, initial_phase)?;

    Ok(())
}
