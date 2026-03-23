---
title: "clc.yml silently controls full worker permission grants — no user review before dispatch"
status: discovery
priority:
assignee:
labels: [clc, security, permissions]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

1. Worker permission grants should be reviewed or at least visible to the user before autonomous agents receive them.
2. `clc.yml` (or `clc.toml`) defines `worker.permissions.default` and `worker.permissions.deny` (parsed in `clc/src/config.rs`, lines 74-87). At dispatch time, `cmd_dispatch` in `main.rs` (line 364) loads the config and passes `cfg.worker.permissions.default` directly to `permissions::seed_defaults` (called at `dispatch.rs` line 72). `seed_defaults` in `clc/src/permissions.rs` (lines 213-284) writes these permissions verbatim into `.claude/settings.local.json` with `defaultMode: "dontAsk"`. A malicious or misconfigured `clc.yml` containing `worker.permissions.default: ["Bash(*)"]` would grant the worker unrestricted bash access with zero user interaction.
3. There is no confirmation prompt, no diff display, no audit log, and no upper bound on what permissions can be granted through config. The config file is version-controlled and could be modified by a prior worker on a feature branch, then merged to main — silently escalating permissions for all future dispatches.

## Open Questions

- Should `seed_defaults` display the effective permission set before writing, or require explicit confirmation for broad patterns like `Bash(*)`?
- Should there be a denylist of permission patterns that config cannot grant (e.g., patterns without path or command constraints)?
- Can a worker on a feature branch modify `clc.yml` on trunk via a merge, thereby escalating its own future permissions?

## Why It Matters

A single line in a config file — potentially contributed by an untrusted branch merge — silently controls what tools an autonomous agent can use without human review. This is the primary permission boundary for dispatched workers, and it has no guardrails.
