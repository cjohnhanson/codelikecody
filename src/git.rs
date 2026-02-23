use std::path::Path;
use std::process::Command;

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
    let branch = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())?;

    let is_main = branch == "main" || branch == "master";

    // In a worktree, .git is a file (not a directory) containing "gitdir: ..."
    let dot_git = cwd.join(".git");
    let is_worktree = dot_git.is_file();

    Some(GitState {
        branch,
        is_main,
        is_worktree,
    })
}
