---
title: "clc init --agent-hooks flag for prime injection and workflow enforcement"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-03T00:48:46Z"
updated: "2026-03-03T00:48:46Z"
---

## Problem

clc tools (clc, tisket, missouri) each have their own `init` command, but none of them currently offer a way to opt into the agent-facing hook system (prime text injection, phase enforcement, trunk guards, etc.) at init time. Setting up a project for agent-driven development requires manually configuring Claude Code hooks after init.

## Design

Add an `--agent-hooks` flag (or similar) to `clc init` that:

1. Installs Claude Code hook configuration into `.claude/settings.json` (or `.claude/settings.local.json`)
2. Configures the hook events: PreToolUse, PostToolUse, SessionStart, Stop, UserPromptSubmit
3. Each hook points to `clc hook <event>` as the command
4. Sets up the baseline `.clc/config.yml` if it doesn't exist

This is the "batteries included" setup for projects that want the full clc workflow enforcement. Without `--agent-hooks`, `clc init` works as it does today — just creates `.clc/config.yml` and optionally `.clc/state`.

### Scope

- `clc init --agent-hooks` — writes hook config into Claude Code settings
- Should be idempotent (safe to run multiple times)
- Should warn if existing hooks would be overwritten (same as current `--force` behavior)
- Consider whether tisket and missouri should also have awareness of this flag, or if it's purely a clc concern

### Open questions

- Should this also seed baseline permissions (`clc permissions seed`)?
- Should `--agent-hooks` be the default when `clc init` detects it's running inside a Claude Code session?
- Flag name: `--agent-hooks`, `--hooks`, `--workflow`, something else?
