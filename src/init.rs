use std::path::Path;

use serde_json::{json, Value};

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

pub fn init(project_dir: &Path) -> Result<(), Error> {
    let clc_command = resolve_hook_command();

    // Create .clc/ state directory
    let clc_dir = project_dir.join(".clc");
    std::fs::create_dir_all(&clc_dir)?;

    // Create .claude/ and write settings.local.json
    let claude_dir = project_dir.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.local.json");
    let settings = if settings_path.exists() {
        let existing: Value = serde_json::from_str(&std::fs::read_to_string(&settings_path)?)?;
        merge_hooks(existing, &generate_settings(&clc_command))
    } else {
        generate_settings(&clc_command)
    };

    let formatted = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&settings_path, formatted)?;

    Ok(())
}

fn resolve_hook_command() -> String {
    std::env::current_exe()
        .map_or_else(|_| "clc hook".to_string(), |p| format!("{} hook", p.display()))
}

fn merge_hooks(mut existing: Value, new: &Value) -> Value {
    if let (Some(existing_obj), Some(new_obj)) = (existing.as_object_mut(), new.as_object())
        && let Some(new_hooks) = new_obj.get("hooks")
    {
        existing_obj.insert("hooks".to_string(), new_hooks.clone());
    }
    existing
}
