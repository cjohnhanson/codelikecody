---
title: "Consolidate external shell hooks into clc hook system with contextual execution"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T19:56:00Z
updated: "2026-03-01T16:52:19Z"
---

## Problem

Hook logic currently lives in two places:

1. **clc Rust hook system** (hook.rs, guard.rs) -- phase guards, trunk write protection, prime text injection, post-tool nudges
2. **External shell scripts** (~/Projects/co.d/claude.d/hooks/) -- general-purpose guards deployed globally via nix/hms

The external hooks run everywhere, on every project, with no awareness of clc state. They are blunt instruments. The clc hook system is context-aware (knows phase, branch, tisket, missouri state) but only enforces clc-specific workflow rules.

## Current external hooks inventory

### PreToolUse
- **git-add-validator.sh** -- blocks git add -A, git add ., directory adds. Forces explicit file staging.
- **python-validator.sh** -- blocks bare python/python3, enforces uv run python
- **coderup-validator.sh** -- ensures login code is provided with coder commands
- **picnic-worktree-guard.sh** -- blocks edits to picnic main repo, forces worktree use

### PostToolUse
- **auto-format.sh** -- runs formatters/linters after Claude edits files

### Stop
- **completion-check.sh** -- blocks stop unless task completion is confirmed

### Shared
- **lib/common.sh** -- JSON parsing helpers (read_json, get_json_field)

## What consolidation means

Move the hook logic that makes sense for clc-managed projects into the Rust hook system. Not necessarily all hooks -- some (like picnic-worktree-guard) are project-specific and belong in project-level config. But the patterns are reusable:

- **Command validators** (git-add, python/uv) could be configurable guards in clc, enabled per-project or globally
- **Auto-format** could be a PostToolUse behavior, aware of which formatter to use based on file type and project config
- **Stop completion check** overlaps with check_stop() -- consolidating means one stop enforcement path instead of two independent ones (and fixing the --print mode gap)

## Contextual execution

The key advantage of moving into clc: hooks become context-aware. Examples:
- Git-add validator only active on branches, not trunk (trunk already blocks writes)
- Auto-format uses project-specific formatter config
- Stop check knows the phase system and can enforce must-reach-done
- Python/uv enforcement only on Python projects (detectable from pyproject.toml or similar)

## What stays external

Some hooks are inherently per-user or cross-project:
- **picnic-worktree-guard** -- specific to one repo, not a general pattern
- **coderup-validator** -- specific to one tool, not a general pattern

These stay as external shell hooks unless a general project-specific-command-validators system emerges in clc.

## Open questions

- Should the hook system support user-defined validators (like a plugin/config system)?
- Or should it just absorb the common patterns (git-add, uv, auto-format, stop) as built-in behavior?
- How does this interact with the claude-code crate extraction -- does the hook system move there or stay in clc?


## Exhibit: observed shell hook unreliability (2026-02-28)

In the same session, two hooks demonstrated opposite failure modes:

1. **False positive**: A heredoc writing tisket content that mentioned a tool name triggered a validator. The hook matched on string content inside a heredoc, not on an actual command invocation.

2. **False negative (or race condition)**: An earlier bare python3 command in this same session was not blocked by the python validator, despite the regex appearing correct. The same command was blocked later. Suggests hook execution order or error handling inconsistency.

Same hook infrastructure, same directory, same session. Brittle behavior. This is exactly why consolidation into a Rust-based system with proper command parsing matters -- shell regex matching on serialized JSON is fundamentally fragile.
