---
title: "BASELINE_PERMISSIONS hardcoded fallback gives over-broad permissions when config absent"
status: todo
priority:
assignee:
labels: [clc, security, permissions]
depends_on: []
created: "2026-03-23T03:12:04Z"
updated: "2026-03-23T03:12:04Z"
---

## Problem

1. When no `worker.permissions.default` is configured, the fallback permission set should be minimal — enough to function, nothing more.
2. `seed_defaults` in `clc/src/permissions.rs` (lines 244-249) falls back to `BASELINE_PERMISSIONS` (lines 110-146) when `config_defaults` is empty. This hardcoded list includes `"Write"`, `"Edit"`, and `"MultiEdit"` without path constraints, `"Bash(cargo *)"` which covers `cargo publish`, and `"Bash(git add *)"` / `"Bash(git commit *)"` without scope limits. The `"WebFetch"` and `"WebSearch"` permissions allow network access. These are granted with `defaultMode: "dontAsk"`, meaning the agent never prompts for confirmation.
3. A project that hasn't configured `worker.permissions.default` in `clc.yml` gets workers with unrestricted file write access across the entire filesystem (not scoped to the worktree), the ability to run any cargo subcommand, and outbound network access — all silently, as a "safe default."

## Open Questions

- Should `BASELINE_PERMISSIONS` scope `Write` and `Edit` to `{worktree}/**` like the config-driven path does?
- Should `Bash(cargo *)` be narrowed to exclude `cargo publish`, `cargo login`, etc.?
- Is the fallback even desirable, or should absence of config mean absence of pre-granted permissions (forcing explicit configuration)?

## Why It Matters

The fallback is the default for every project that hasn't opted into explicit permission configuration. Broad defaults applied silently undermine the permission system's purpose — users who haven't configured permissions likely assume workers have restricted access, not unrestricted write and network capabilities.
