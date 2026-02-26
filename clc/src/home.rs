use std::path::{Path, PathBuf};

use crate::error::Error;

/// Resolve the main repository root path.
/// For a linked worktree, returns the parent of the shared `.git` directory.
/// For the main worktree, returns the repo root directly.
pub fn home(project_dir: &Path) -> Result<PathBuf, Error> {
    let repo = gix::discover(project_dir)
        .map_err(|_| Error::NonBlocking("not inside a git repository".to_string()))?;

    // common_dir points to the shared `.git` directory (may contain `..` components).
    // Canonicalize to resolve symlinks and relative segments, then take its parent.
    let common_dir = repo
        .common_dir()
        .canonicalize()
        .map_err(|e| Error::NonBlocking(format!("failed to resolve git common dir: {e}")))?;

    common_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::NonBlocking("could not determine main worktree".to_string()))
}
