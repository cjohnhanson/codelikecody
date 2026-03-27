use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::error::Error;

const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "TeammateIdle",
    "TaskCompleted",
    "PreCompact",
    "SessionEnd",
];

pub fn generate_settings(clc_command: &str) -> Value {
    let mut hooks = serde_json::Map::new();
    for event in HOOK_EVENTS {
        hooks.insert(
            (*event).to_string(),
            json!([{
                "hooks": [{
                    "type": "command",
                    "command": clc_command
                }]
            }]),
        );
    }
    json!({ "hooks": hooks })
}

pub fn init(project_dir: &Path, untracked: bool, force: bool) -> Result<(), Error> {
    let clc_command = resolve_hook_command();

    // Create .clc/ state directory
    let clc_dir = project_dir.join(".clc");
    std::fs::create_dir_all(&clc_dir)?;

    // Create .claude/ and write settings.local.json
    let claude_dir = project_dir.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.local.json");
    let new_settings = generate_settings(&clc_command);

    let settings = if settings_path.exists() {
        let existing: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?;
        if !force && has_existing_hooks(&existing) {
            return Err(Error::Block(
                "Existing hooks found in .claude/settings.local.json.\n\
                 Use --force to overwrite them."
                    .to_string(),
            ));
        }
        merge_hooks(existing, &new_settings)
    } else {
        new_settings
    };

    let formatted = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&settings_path, formatted)?;

    if untracked {
        write_git_excludes(project_dir)?;
    }

    Ok(())
}

/// Initialize user-level config at `$HOME/.clc/config.yml`.
/// Creates the config file with default contents if it doesn't exist.
/// Idempotent — does nothing if the file already exists.
pub fn init_user() -> Result<(), Error> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| Error::Block("HOME environment variable not set".to_string()))?;

    let clc_dir = home.join(".clc");
    std::fs::create_dir_all(&clc_dir)?;

    let config_path = clc_dir.join("config.yml");
    if !config_path.exists() {
        std::fs::write(
            &config_path,
            "skills: []\n",
        )?;
    }

    eprintln!("initialized user-level clc config at {}", config_path.display());
    Ok(())
}

fn has_existing_hooks(settings: &Value) -> bool {
    let Some(hooks) = settings.get("hooks").and_then(Value::as_object) else {
        return false;
    };
    // Only counts as "existing" if there are non-clc hooks present.
    hooks.values().any(|event_array| {
        event_array
            .as_array()
            .is_some_and(|arr| arr.iter().any(|matcher| !is_clc_matcher(matcher)))
    })
}

const EXCLUDE_PATTERNS: &[&str] = &[
    ".clc/",
    ".claude/settings.local.json",
    "tisket.yml",
    ".tisket/",
    "zettel.yml",
    ".zettel/",
];

fn write_git_excludes(project_dir: &Path) -> Result<(), Error> {
    let exclude_path = project_dir.join(".git").join("info").join("exclude");
    let info_dir = exclude_path.parent().expect("exclude has parent");

    if !project_dir.join(".git").is_dir() {
        return Err(Error::NonBlocking(
            "--untracked requires a git repository".into(),
        ));
    }

    std::fs::create_dir_all(info_dir)?;

    let existing = if exclude_path.exists() {
        std::fs::read_to_string(&exclude_path)?
    } else {
        String::new()
    };

    let mut content = existing;
    for pattern in EXCLUDE_PATTERNS {
        if !content.lines().any(|line| line == *pattern) {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(pattern);
            content.push('\n');
        }
    }

    std::fs::write(&exclude_path, content)?;
    Ok(())
}

fn resolve_hook_command() -> String {
    // Use bare `clc hook` rather than an absolute path so the hook works
    // both on the host and inside Docker containers where the binary
    // is at a different path.
    "clc hook".to_string()
}

/// Check whether a matcher object contains a clc hook command.
fn is_clc_matcher(matcher: &Value) -> bool {
    matcher
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|cmd| cmd.contains("clc") && cmd.contains("hook"))
            })
        })
}

fn merge_hooks(mut existing: Value, new: &Value) -> Value {
    let (Some(existing_obj), Some(new_obj)) = (existing.as_object_mut(), new.as_object()) else {
        return existing;
    };
    let Some(new_hooks) = new_obj.get("hooks").and_then(Value::as_object) else {
        return existing;
    };

    let existing_hooks = existing_obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("hooks is an object");

    for (event, new_matchers) in new_hooks {
        let Some(new_arr) = new_matchers.as_array() else {
            continue;
        };

        let event_arr = existing_hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .expect("event is an array");

        // Strip any previous clc matchers (handles re-init / upgrades).
        event_arr.retain(|m| !is_clc_matcher(m));

        // Append the new clc matchers.
        event_arr.extend(new_arr.iter().cloned());
    }

    existing
}
