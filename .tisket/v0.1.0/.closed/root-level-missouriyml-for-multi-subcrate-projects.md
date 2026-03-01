---
title: "Root-level missouri.yml for multi-subcrate projects"
status: done
priority: 2
assignee:
labels: [missouri]
depends_on: []
created: 2026-02-26T04:24:26Z
updated: "2026-03-01T01:04:15Z"
---

This repo has missouri test suites in subcrates (`tisket/tests/missouri/`,
`clc/tests/missouri/`) but nothing at the root. The clc reinforcement hook
reports "missouri: no tests" because it looks at the worktree/project root
and finds no `.missouri/` directory.

A root-level `missouri.yml` (or `.missouri/missouri.yml`) should know about
the subcrate test suites and aggregate them. `missouri run` from the root
should run all of them and report a unified result.

This also solves the reinforcement gap — once the root has a missouri config,
the hook detects it and the status line reflects actual test state instead of
"no tests."

## Scratch Notes

### Implementation Summary
- Added `members: Vec<Utf8PathBuf>` to `ProjectConfig` in config.rs
- Added `load_workspace_members()` to graph.rs — resolves root missouri.yml, returns member paths if `members` is non-empty
- Added workspace routing in cli.rs: `run`, `list`, `validate` all check for workspace members before normal discovery
- Three new internal functions: `run_workspace_members`, `list_workspace_members`, `validate_workspace_members`
- `member_label()` uses `strip_prefix(workspace_root)` to produce relative path labels (e.g., `clc/tests/missouri`)
- Root `missouri.yml` created with `members: [clc/tests/missouri, tisket/tests/missouri]`

### Test Coverage
- 3 config parsing unit tests for members field
- 5 CLI integration tests using fixtures 19-workspace and 20-workspace-fail
- All 129 missouri tests pass

### Commits
1. `feat(missouri): workspace mode with members for multi-project aggregation` — TDD tests + implementation
2. `fix(missouri): use relative paths for workspace member labels` — label fix + root missouri.yml

### Note
tisket/tests/missouri tests fail independently (tisket init issue in fixture env), unrelated to workspace mode.
