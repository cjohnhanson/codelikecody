//! Validates `git add` commands to prevent bulk staging.
//!
//! Blocks: `git add -A`, `git add .`, `git add *`, `git add -u`,
//! and `git add <directory>`. Allows individual file paths.

use std::path::Path;

/// Result of validating a `git add` command.
#[derive(Debug, PartialEq)]
pub enum GitAddCheck {
    /// Not a git add command — no opinion.
    NotGitAdd,
    /// Valid git add with individual file paths.
    Allowed,
    /// Blocked with a reason.
    Blocked(String),
}

/// Validate a shell command string for bulk `git add` usage.
///
/// The `cwd` is used to resolve relative paths for directory checks.
pub fn validate(command: &str, cwd: &Path) -> GitAddCheck {
    // Split on command separators to isolate individual commands.
    // This handles `git add file && git commit` without treating
    // commit args as git-add args.
    for subcmd in split_commands(command) {
        let trimmed = subcmd.trim();
        if !is_git_add(trimmed) {
            continue;
        }
        let args = extract_git_add_args(trimmed);
        if let Some(reason) = check_args(&args, cwd) {
            return GitAddCheck::Blocked(reason);
        }
    }

    // If we found at least one git add and none were blocked, it's allowed.
    // If we found no git add at all, it's not our concern.
    if split_commands(command).any(|s| is_git_add(s.trim())) {
        GitAddCheck::Allowed
    } else {
        GitAddCheck::NotGitAdd
    }
}

/// Check if a (sub)command starts with `git add`.
fn is_git_add(cmd: &str) -> bool {
    let normalized = cmd.split_whitespace().collect::<Vec<_>>();
    normalized.len() >= 2 && normalized[0] == "git" && normalized[1] == "add"
}

/// Extract arguments after `git add`, skipping known flags.
fn extract_git_add_args(cmd: &str) -> Vec<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    // Skip "git" and "add"
    let mut args = Vec::new();
    let mut iter = parts.iter().skip(2);
    let mut past_separator = false;

    while let Some(part) = iter.next() {
        if *part == "--" {
            past_separator = true;
            continue;
        }

        if !past_separator && part.starts_with('-') {
            // Known flags that take a value — skip the next token too.
            if matches!(*part, "--chmod" | "--pathspec-from-file") {
                iter.next();
            }
            // All other flags are boolean — just skip the flag itself.
            continue;
        }

        // Strip surrounding quotes.
        let cleaned = part.trim_matches(|c| c == '"' || c == '\'');
        if !cleaned.is_empty() {
            args.push(cleaned.to_string());
        }
    }
    args
}

/// Check extracted args for bulk-add patterns. Returns the block reason
/// or None if all args are valid file paths.
fn check_args(args: &[String], cwd: &Path) -> Option<String> {
    if args.is_empty() {
        return Some("git add with no paths — stage files individually".to_string());
    }

    for arg in args {
        // Exact blocklist.
        match arg.as_str() {
            "." => return Some("git add . stages everything — stage files individually".to_string()),
            "-A" | "--all" => return Some("git add -A stages everything — stage files individually".to_string()),
            "-u" => return Some("git add -u stages all tracked changes — stage files individually".to_string()),
            _ => {}
        }

        // Glob patterns.
        if arg.contains('*') || arg.contains('?') {
            return Some(format!("git add with glob '{arg}' — stage files individually"));
        }

        // Directory check: resolve against cwd.
        let path = cwd.join(arg);
        if path.is_dir() {
            return Some(format!("git add targets directory '{arg}' — stage files individually"));
        }
    }

    None
}

/// Split a command string on `&&`, `||`, and `;` separators.
fn split_commands(command: &str) -> impl Iterator<Item = &str> {
    // Simple split — doesn't handle these separators inside quotes or
    // heredocs, but covers the common chained-command patterns.
    command
        .split("&&")
        .flat_map(|s| s.split("||"))
        .flat_map(|s| s.split(';'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        // Create a directory and a file for path resolution tests.
        fs::create_dir(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("README.md"), "# hello").unwrap();
        tmp
    }

    // --- Not git add: passthrough ---

    #[test]
    fn non_git_command_is_not_git_add() {
        let tmp = setup_dir();
        assert_eq!(validate("cargo build", tmp.path()), GitAddCheck::NotGitAdd);
    }

    #[test]
    fn git_commit_is_not_git_add() {
        let tmp = setup_dir();
        assert_eq!(validate("git commit -m 'foo'", tmp.path()), GitAddCheck::NotGitAdd);
    }

    #[test]
    fn git_status_is_not_git_add() {
        let tmp = setup_dir();
        assert_eq!(validate("git status", tmp.path()), GitAddCheck::NotGitAdd);
    }

    // --- Blocked: bulk patterns ---

    #[test]
    fn blocks_git_add_all_flag() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add -A", tmp.path()), GitAddCheck::Blocked(_)));
    }

    #[test]
    fn blocks_git_add_all_long_flag() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add --all", tmp.path()), GitAddCheck::Blocked(_)));
    }

    #[test]
    fn blocks_git_add_dot() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add .", tmp.path()), GitAddCheck::Blocked(_)));
    }

    #[test]
    fn blocks_git_add_glob() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add *.rs", tmp.path()), GitAddCheck::Blocked(_)));
    }

    #[test]
    fn blocks_git_add_u() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add -u", tmp.path()), GitAddCheck::Blocked(_)));
    }

    #[test]
    fn blocks_git_add_directory() {
        let tmp = setup_dir();
        assert!(matches!(validate("git add src", tmp.path()), GitAddCheck::Blocked(_)));
    }

    // --- Allowed: individual files ---

    #[test]
    fn allows_single_file() {
        let tmp = setup_dir();
        assert_eq!(validate("git add README.md", tmp.path()), GitAddCheck::Allowed);
    }

    #[test]
    fn allows_file_in_subdirectory() {
        let tmp = setup_dir();
        assert_eq!(validate("git add src/main.rs", tmp.path()), GitAddCheck::Allowed);
    }

    #[test]
    fn allows_multiple_explicit_files() {
        let tmp = setup_dir();
        assert_eq!(
            validate("git add README.md src/main.rs", tmp.path()),
            GitAddCheck::Allowed,
        );
    }

    #[test]
    fn allows_file_with_force_flag() {
        let tmp = setup_dir();
        assert_eq!(validate("git add -f README.md", tmp.path()), GitAddCheck::Allowed);
    }

    #[test]
    fn allows_file_after_double_dash() {
        let tmp = setup_dir();
        assert_eq!(validate("git add -- README.md", tmp.path()), GitAddCheck::Allowed);
    }

    // --- Chained commands ---

    #[test]
    fn validates_git_add_in_chain() {
        let tmp = setup_dir();
        assert_eq!(
            validate("git add README.md && git commit -m 'update'", tmp.path()),
            GitAddCheck::Allowed,
        );
    }

    #[test]
    fn blocks_bulk_add_in_chain() {
        let tmp = setup_dir();
        assert!(matches!(
            validate("git add . && git commit -m 'bad'", tmp.path()),
            GitAddCheck::Blocked(_),
        ));
    }

    #[test]
    fn chain_with_no_git_add_is_not_git_add() {
        let tmp = setup_dir();
        assert_eq!(
            validate("cargo build && cargo test", tmp.path()),
            GitAddCheck::NotGitAdd,
        );
    }

    // --- The original bug: file path traversing a directory ---

    #[test]
    fn allows_file_path_through_directory() {
        let tmp = setup_dir();
        // src/ is a directory, but src/main.rs is a file.
        // The old shell hook would block this because "src" is a directory.
        assert_eq!(validate("git add src/main.rs", tmp.path()), GitAddCheck::Allowed);
    }

    // --- Symlink to directory ---

    #[test]
    fn blocks_symlink_to_directory() {
        let tmp = setup_dir();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(tmp.path().join("src"), tmp.path().join("link-to-src"))
                .unwrap();
            assert!(matches!(
                validate("git add link-to-src", tmp.path()),
                GitAddCheck::Blocked(_),
            ));
        }
    }

    // --- Nonexistent path (file that doesn't exist yet) ---

    #[test]
    fn allows_nonexistent_path() {
        let tmp = setup_dir();
        // New files that don't exist on disk yet should be allowed —
        // git add will stage them if they're in the index.
        assert_eq!(validate("git add brand-new-file.rs", tmp.path()), GitAddCheck::Allowed);
    }
}
