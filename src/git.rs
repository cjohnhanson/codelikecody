use std::path::Path;

/// Git state detected from the working directory.
#[derive(Debug)]
pub struct GitState {
    pub branch: String,
    pub is_main: bool,
    pub is_worktree: bool,
}

/// Detect git state from the given directory.
/// Returns `None` if not inside a git repository.
pub fn detect(cwd: &Path) -> Option<GitState> {
    let repo = gix::discover(cwd).ok()?;

    let head = repo.head().ok()?;
    let branch = head.referent_name()?.shorten().to_string();

    let is_main = branch == "main" || branch == "master";
    let is_worktree = repo.kind() == gix::repository::Kind::LinkedWorkTree;

    Some(GitState {
        branch,
        is_main,
        is_worktree,
    })
}
