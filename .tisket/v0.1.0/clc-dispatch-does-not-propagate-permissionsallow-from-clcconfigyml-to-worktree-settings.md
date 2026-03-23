---
title: "clc dispatch does not propagate permissions.allow from .clc/config.yml to worktree settings"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-20T00:39:30Z
updated: "2026-03-23T02:14:08Z"
---

## Problem

The `clc.yml` config supports a `permissions.allow` field (defined as `PermissionsConfig` in `config.rs`) which should let projects declare additional permissions that workers receive at dispatch time.

In `cmd_dispatch` (main.rs line 362), dispatch is called with `cfg.worker.permissions.default` and `cfg.worker.permissions.deny` — these come from the `worker.permissions` config section. The top-level `permissions.allow` field from `cfg.permissions.allow` is never read or passed to `dispatch::dispatch()` or `permissions::seed_defaults()`. The two config sections (`permissions.allow` and `worker.permissions.default`) serve overlapping purposes but only the worker-scoped one actually takes effect.

A user who adds `permissions.allow: ["Bash(just *)"]` to their `clc.yml` (a valid and parseable config) would expect workers to receive that permission. They don't — the permission is parsed, serialized in `clc config show`, but silently dropped on the floor during dispatch.

## Open Questions

- Should `permissions.allow` be merged with `worker.permissions.default` at dispatch time, or should one of these config fields be deprecated?
- Is `permissions.allow` intended for a different purpose than worker permissions (e.g., coordinator permissions, or hook-level permissions)?
- What's the correct precedence if both fields are set?

## Why It Matters

Silent config that parses correctly but has no effect is worse than a missing feature — it gives users false confidence that their permission configuration is being applied when workers are actually running with different permissions than intended.
