use std::path::Path;

/// Git state detected from the working directory.
#[derive(Debug)]
pub struct GitState {
    pub branch: String,
    pub is_main: bool,
    pub is_admin: bool,
    pub is_worktree: bool,
}

/// Detect git state from the given directory.
/// Returns `None` if not inside a git repository.
///
/// The `main_branch` parameter specifies the configured main branch name.
/// The branch is considered "main" if it matches `main_branch` or `master`.
/// The `admin_branch` parameter specifies the configured admin branch name.
pub fn detect(cwd: &Path, main_branch: &str, admin_branch: &str) -> Option<GitState> {
    let repo = gix::discover(cwd).ok()?;

    let head = repo.head().ok()?;
    let branch = head.referent_name()?.shorten().to_string();

    let is_main = branch == main_branch || branch == "master";
    let is_admin = branch == admin_branch;
    let is_worktree = repo.kind() == gix::repository::Kind::LinkedWorkTree;

    Some(GitState {
        branch,
        is_main,
        is_admin,
        is_worktree,
    })
}

/// Return the current branch name, or empty string if not in a git repo.
pub fn current_branch(cwd: &Path) -> Option<String> {
    let repo = gix::discover(cwd).ok()?;
    let head = repo.head().ok()?;
    Some(head.referent_name()?.shorten().to_string())
}
