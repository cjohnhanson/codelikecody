use std::path::Path;

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
        write_untracked_state(project_dir)?;
    }

    Ok(())
}

fn has_existing_hooks(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|h| !h.is_empty())
}

const EXCLUDE_PATTERNS: &[&str] = &[
    ".clc/",
    ".claude/settings.local.json",
    "tisket.yml",
    ".tisket/",
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

fn write_untracked_state(project_dir: &Path) -> Result<(), Error> {
    let state_path = project_dir.join(".clc").join("state");

    let mut content = if state_path.exists() {
        std::fs::read_to_string(&state_path)?
    } else {
        String::new()
    };

    if !content.lines().any(|line| line.starts_with("untracked:")) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("untracked: true\n");
    }

    std::fs::write(&state_path, content)?;
    Ok(())
}

fn resolve_hook_command() -> String {
    std::env::current_exe().map_or_else(
        |_| "clc hook".to_string(),
        |p| format!("{} hook", p.display()),
    )
}

fn merge_hooks(mut existing: Value, new: &Value) -> Value {
    if let (Some(existing_obj), Some(new_obj)) = (existing.as_object_mut(), new.as_object())
        && let Some(new_hooks) = new_obj.get("hooks")
    {
        existing_obj.insert("hooks".to_string(), new_hooks.clone());
    }
    existing
}
