use std::path::Path;

use crate::config::SkillSource;
use crate::error::Error;

/// A single skill entry: name, description, and where to find the full content.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub source: SkillLocation,
}

/// Where the full SKILL.md content lives.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillLocation {
    /// On-disk path to the SKILL.md file.
    File(String),
    /// Built into the binary; use `clc skills show <name>`.
    #[allow(dead_code)] // Populated when built-in skill content is authored.
    BuiltIn,
}

/// YAML frontmatter parsed from a SKILL.md file.
#[derive(Debug, serde::Deserialize)]
struct SkillFrontmatter {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// Scan all configured skill sources and return an index of available skills.
pub fn index(project_dir: &Path, sources: &[SkillSource]) -> Vec<SkillEntry> {
    let mut entries = Vec::new();

    // Built-in skills always included.
    entries.extend(builtin_skills());

    for source in sources {
        match source {
            SkillSource::Path { path } => {
                let resolved = resolve_path(project_dir, path);
                if let Ok(found) = scan_directory(&resolved) {
                    entries.extend(found);
                }
            }
            SkillSource::Git { git: _ } => {
                // Git sources require a local clone. Not yet implemented.
            }
        }
    }

    entries
}

/// Scan a directory for subdirectories containing SKILL.md and parse frontmatter.
fn scan_directory(dir: &Path) -> Result<Vec<SkillEntry>, Error> {
    let mut entries = Vec::new();

    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| Error::NonBlocking(format!("failed to read skill dir {}: {e}", dir.display())))?;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        if let Ok(entry) = parse_skill_md(&skill_md, &path) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Parse YAML frontmatter from a SKILL.md file.
fn parse_skill_md(skill_md: &Path, skill_dir: &Path) -> Result<SkillEntry, Error> {
    let content = std::fs::read_to_string(skill_md)
        .map_err(|e| Error::NonBlocking(format!("failed to read {}: {e}", skill_md.display())))?;

    let frontmatter = extract_frontmatter(&content)
        .ok_or_else(|| Error::NonBlocking(format!("no frontmatter in {}", skill_md.display())))?;

    let parsed: SkillFrontmatter = serde_yml::from_str(frontmatter)
        .map_err(|e| Error::NonBlocking(format!("invalid frontmatter in {}: {e}", skill_md.display())))?;

    let dir_name = skill_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    Ok(SkillEntry {
        name: parsed.name.unwrap_or_else(|| dir_name.to_string()),
        description: parsed.description.unwrap_or_default(),
        source: SkillLocation::File(skill_md.to_string_lossy().into_owned()),
    })
}

/// Extract YAML frontmatter between `---` delimiters.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    Some(rest[..end].trim())
}

/// Resolve a skill path relative to the project directory.
/// Handles `~` expansion and relative paths.
fn resolve_path(project_dir: &Path, path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        project_dir.join(path)
    }
}

/// Return built-in skills compiled into the binary.
fn builtin_skills() -> Vec<SkillEntry> {
    // Placeholder — built-in skills will be added as content is authored.
    Vec::new()
}

/// Format the skill index for injection into prime text.
pub fn format_index(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Skills (clc)\n\nAvailable skills — read the full SKILL.md when needed:\n\n");
    for entry in entries {
        let retrieval = match &entry.source {
            SkillLocation::File(path) => format!("file: {path}"),
            SkillLocation::BuiltIn => format!("`clc skills show {}`", entry.name),
        };
        out.push_str(&format!("- **{}**: {} ({})\n", entry.name, entry.description, retrieval));
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\nname: my-skill\ndescription: Does a thing\n---\n\n# My Skill\n";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("name: my-skill"));
        assert!(fm.contains("description: Does a thing"));
    }

    #[test]
    fn extract_frontmatter_missing_returns_none() {
        let content = "# No frontmatter here\nJust markdown.\n";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn extract_frontmatter_incomplete_returns_none() {
        let content = "---\nname: incomplete\n";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn parse_skill_md_extracts_name_and_description() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Does a useful thing\n---\n\n# Instructions\n",
        )
        .unwrap();

        let entry = parse_skill_md(&skill_dir.join("SKILL.md"), &skill_dir).unwrap();
        assert_eq!(entry.name, "my-skill");
        assert_eq!(entry.description, "Does a useful thing");
    }

    #[test]
    fn parse_skill_md_falls_back_to_dir_name_when_no_name_field() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("fallback-name");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\ndescription: Just a description\n---\n\nContent\n",
        )
        .unwrap();

        let entry = parse_skill_md(&skill_dir.join("SKILL.md"), &skill_dir).unwrap();
        assert_eq!(entry.name, "fallback-name");
    }

    #[test]
    fn scan_directory_finds_skills() {
        let dir = tempfile::tempdir().unwrap();

        // Create two skill directories
        let skill_a = dir.path().join("skill-a");
        std::fs::create_dir_all(&skill_a).unwrap();
        std::fs::write(
            skill_a.join("SKILL.md"),
            "---\nname: skill-a\ndescription: First skill\n---\n\nContent\n",
        )
        .unwrap();

        let skill_b = dir.path().join("skill-b");
        std::fs::create_dir_all(&skill_b).unwrap();
        std::fs::write(
            skill_b.join("SKILL.md"),
            "---\nname: skill-b\ndescription: Second skill\n---\n\nContent\n",
        )
        .unwrap();

        // Create a non-skill directory (no SKILL.md)
        let not_a_skill = dir.path().join("not-a-skill");
        std::fs::create_dir_all(&not_a_skill).unwrap();
        std::fs::write(not_a_skill.join("README.md"), "just a readme").unwrap();

        let entries = scan_directory(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"skill-a"));
        assert!(names.contains(&"skill-b"));
    }

    #[test]
    fn scan_directory_skips_files_at_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SKILL.md"), "---\nname: root\n---\n").unwrap();

        let entries = scan_directory(dir.path()).unwrap();
        assert!(entries.is_empty(), "SKILL.md at root level should be ignored — only subdirectories");
    }

    #[test]
    fn scan_directory_nonexistent_returns_error() {
        let result = scan_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn index_with_local_path_source() {
        let dir = tempfile::tempdir().unwrap();

        // Project dir
        let project = dir.path().join("project");
        std::fs::create_dir_all(&project).unwrap();

        // Skills dir with one skill
        let skills_dir = dir.path().join("skills");
        let skill = skills_dir.join("my-skill");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill\n---\n\nContent\n",
        )
        .unwrap();

        let sources = vec![SkillSource::Path {
            path: skills_dir.to_string_lossy().into_owned(),
        }];

        let entries = index(&project, &sources);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "my-skill");
        assert_eq!(entries[0].description, "A test skill");
    }

    #[test]
    fn index_with_relative_path_source() {
        let dir = tempfile::tempdir().unwrap();

        // Project dir with a skills/ subdirectory
        let skill = dir.path().join("skills").join("local-skill");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: local-skill\ndescription: A local skill\n---\n\nContent\n",
        )
        .unwrap();

        let sources = vec![SkillSource::Path {
            path: "./skills/".into(),
        }];

        let entries = index(dir.path(), &sources);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "local-skill");
    }

    #[test]
    fn index_with_no_sources_returns_only_builtins() {
        let dir = tempfile::tempdir().unwrap();
        let entries = index(dir.path(), &[]);
        // Currently no built-ins, so empty
        assert!(entries.is_empty());
    }

    #[test]
    fn index_skips_missing_path_sources_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let sources = vec![SkillSource::Path {
            path: "/nonexistent/skills/".into(),
        }];

        let entries = index(dir.path(), &sources);
        // Should not panic, just return empty
        assert!(entries.is_empty());
    }

    #[test]
    fn format_index_empty_returns_empty_string() {
        assert_eq!(format_index(&[]), "");
    }

    #[test]
    fn format_index_includes_name_and_description() {
        let entries = vec![
            SkillEntry {
                name: "missouri-authoring".into(),
                description: "How to write missouri tests".into(),
                source: SkillLocation::BuiltIn,
            },
            SkillEntry {
                name: "my-skill".into(),
                description: "A custom skill".into(),
                source: SkillLocation::File("/path/to/my-skill/SKILL.md".into()),
            },
        ];

        let output = format_index(&entries);
        assert!(output.contains("## Skills (clc)"));
        assert!(output.contains("missouri-authoring"));
        assert!(output.contains("How to write missouri tests"));
        assert!(output.contains("`clc skills show missouri-authoring`"));
        assert!(output.contains("my-skill"));
        assert!(output.contains("file: /path/to/my-skill/SKILL.md"));
    }

    #[test]
    fn resolve_path_relative() {
        let project = Path::new("/home/user/project");
        let resolved = resolve_path(project, "./skills/");
        assert_eq!(resolved, Path::new("/home/user/project/./skills/"));
    }

    #[test]
    fn resolve_path_absolute() {
        let project = Path::new("/home/user/project");
        let resolved = resolve_path(project, "/opt/skills/");
        assert_eq!(resolved, Path::new("/opt/skills/"));
    }
}
