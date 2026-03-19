---
title: "Switch clc config to YAML as primary format"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-18T02:51:43Z
updated: "2026-03-19T02:40:14Z"
---

## Scratch Notes

## Goal
Switch clc config so `clc.yml` at project root is the primary format (was `clc.toml`).
Load order: `clc.yml` → `clc.toml` → `.clc/config.yml` → defaults.
`clc config show` should output YAML (was TOML).

## Key file: clc/src/config.rs
- `load()` currently prefers `clc.toml`, falls back to `.clc/config.yml`
- `show()` currently outputs TOML via `toml::to_string_pretty`
- YAML `Config` struct is already serde_yml compatible (flat structure, same as `.clc/config.yml`)
- TOML file has `[project]` wrapper section; YAML is flat

## Implementation changes needed
1. Add `YAML_ROOT_CONFIG_FILENAME: &str = "clc.yml"` constant
2. Update `load()`: check `clc.yml` first, then `clc.toml`, then `.clc/config.yml`
3. Update `show()`: use `serde_yml::to_string` instead of `toml::to_string_pretty`

## Tests written
### New unit tests (config.rs)
- `load_yaml_root_config_from_project_root`
- `load_yaml_root_config_prefers_over_toml`
- `load_yaml_root_config_prefers_over_clc_dir_yaml`
- `load_yaml_root_config_error_on_invalid_yaml`
- `load_yaml_root_config_with_coordinator_section`
- `show_outputs_yaml_format`

### New Missouri states
- `clc/tests/missouri/has-yaml-root-config/` — clc.yml at project root, assertions verify read
- `clc/tests/missouri/yaml-root-config-prefers-over-toml/` — both exist, yaml wins

### Updated Missouri states
- `has-config/.missouri/missouri.yml` — changed grep from `main_branch = .trunk.` to `main_branch: trunk`
- `coordinator-policy-config/.missouri/missouri.yml` — changed grep from `main_branch = .main.` to `main_branch: main`

## Status: implementing
- Phase advanced to tests-written
- Permission requested for .claude/settings.local.json in test states (may not be needed)
- Need to implement config.rs changes
