//! Reviewer resolution: name → `.clc/reviewers/<name>.md` → AgentSpec + prompt.
//!
//! Reviewers are markdown files with AgentSpec frontmatter. The body is the
//! review prompt. Coordinators reference reviewers by name in their config;
//! the runtime resolves the name to a file, parses it, and launches a
//! review agent session.

use std::path::Path;

use clc_sdk::agent_spec::AgentSpec;

use crate::error::Error;

/// Directory under the project root where reviewer files live.
const REVIEWERS_DIR: &str = ".clc/reviewers";

/// A parsed reviewer: agent configuration plus review prompt.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read by coordinator when launching review sessions.
pub struct Reviewer {
    pub name: String,
    pub spec: AgentSpec,
    pub prompt: String,
}

/// Resolve a reviewer name to a parsed `Reviewer`.
///
/// Looks for `.clc/reviewers/<name>.md` relative to `project_dir`.
/// The file is parsed as AgentSpec frontmatter + markdown body.
pub fn resolve(project_dir: &Path, name: &str) -> Result<Reviewer, Error> {
    let path = project_dir.join(REVIEWERS_DIR).join(format!("{name}.md"));

    if !path.exists() {
        return Err(Error::NonBlocking(format!(
            "reviewer '{name}' not found at {}",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        Error::NonBlocking(format!("failed to read reviewer '{name}': {e}"))
    })?;

    let (spec, prompt) = AgentSpec::from_markdown(&content).map_err(|e| {
        Error::NonBlocking(format!("invalid reviewer '{name}': {e}"))
    })?;

    if prompt.trim().is_empty() {
        return Err(Error::NonBlocking(format!(
            "reviewer '{name}' has no prompt (markdown body is empty)"
        )));
    }

    Ok(Reviewer {
        name: name.to_string(),
        spec,
        prompt,
    })
}

/// Resolve all reviewers for a coordinator scope. Returns errors for
/// any that can't be found or parsed.
#[allow(dead_code)] // Used by future workflow runtime.
pub fn resolve_all(
    project_dir: &Path,
    names: &[String],
) -> Result<Vec<Reviewer>, Error> {
    names.iter().map(|name| resolve(project_dir, name)).collect()
}

/// List available reviewer names by scanning `.clc/reviewers/`.
#[allow(dead_code)] // Used by future `clc reviewers list` command.
pub fn list(project_dir: &Path) -> Vec<String> {
    let dir = project_dir.join(REVIEWERS_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "md")
        })
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_reviewer(dir: &Path, name: &str, content: &str) {
        let reviewers_dir = dir.join(REVIEWERS_DIR);
        std::fs::create_dir_all(&reviewers_dir).unwrap();
        std::fs::write(reviewers_dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn resolve_parses_frontmatter_and_body() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(
            dir.path(),
            "test-quality",
            "---\nmodel: sonnet\nmax_turns: 5\n---\n\nCheck that tests are meaningful.\n",
        );

        let r = resolve(dir.path(), "test-quality").unwrap();
        assert_eq!(r.name, "test-quality");
        assert_eq!(r.spec.model.as_deref(), Some("sonnet"));
        assert_eq!(r.spec.max_turns, Some(5));
        assert!(r.prompt.contains("tests are meaningful"));
    }

    #[test]
    fn resolve_no_frontmatter_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(dir.path(), "simple", "Just review this.\n");

        let r = resolve(dir.path(), "simple").unwrap();
        assert!(r.spec.model.is_none());
        assert_eq!(r.prompt.trim(), "Just review this.");
    }

    #[test]
    fn resolve_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve(dir.path(), "nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"), "expected 'not found', got: {err}");
    }

    #[test]
    fn resolve_empty_prompt_errors() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(dir.path(), "empty", "---\nmodel: haiku\n---\n");

        let result = resolve(dir.path(), "empty");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("no prompt"), "expected 'no prompt', got: {err}");
    }

    #[test]
    fn resolve_all_collects_reviewers() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(dir.path(), "alpha", "Review alpha.\n");
        write_reviewer(dir.path(), "beta", "Review beta.\n");

        let reviewers =
            resolve_all(dir.path(), &["alpha".into(), "beta".into()]).unwrap();
        assert_eq!(reviewers.len(), 2);
        assert_eq!(reviewers[0].name, "alpha");
        assert_eq!(reviewers[1].name, "beta");
    }

    #[test]
    fn resolve_all_fails_on_missing() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(dir.path(), "exists", "Present.\n");

        let result = resolve_all(dir.path(), &["exists".into(), "gone".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn list_finds_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        write_reviewer(dir.path(), "code-quality", "Check quality.\n");
        write_reviewer(dir.path(), "scope-check", "Check scope.\n");

        let names = list(dir.path());
        assert_eq!(names, vec!["code-quality", "scope-check"]);
    }

    #[test]
    fn list_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let names = list(dir.path());
        assert!(names.is_empty());
    }
}
