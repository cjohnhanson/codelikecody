---
title: "CLC_GUARD_OFF env var disables all guards — agent on feature branch could self-deescalate"
status: todo
priority:
assignee:
labels: [clc, security, guard]
depends_on: []
created: "2026-03-23T03:12:04Z"
updated: "2026-03-23T03:12:04Z"
---

## Problem

1. Guard bypass mechanisms should not be controllable by the agents the guards are meant to constrain.
2. `evaluate` in `clc/src/guard.rs` (lines 52-54) checks `std::env::var("CLC_GUARD_OFF")` and returns `Response::Passthrough` if the variable is set to any non-empty value. An agent with `Bash(*)` permissions — or even `Bash(export *)` — can run `export CLC_GUARD_OFF=1` to disable all guard checks for subsequent tool invocations in the same process. The guard checks phase enforcement, trunk write protection, and the bash allowlist — all bypassed by a single environment variable.
3. A worker on a feature branch that hits a phase restriction (e.g., trying to edit source files during `tests-unwritten`) can set `CLC_GUARD_OFF=1` and proceed unconstrained. The bypass is invisible to the coordinator — no log entry, no audit trail, no way to detect it happened.

## Open Questions

- Should the escape hatch be removed entirely, or moved to a mechanism the agent cannot control (e.g., a file written by the parent process, checked at startup)?
- Could the guard read the env var once at process startup and ignore subsequent changes?
- Is there a way to detect and alert when `CLC_GUARD_OFF` is set during a worker session?

## Why It Matters

The guard system is the behavioral enforcement layer for the entire TDD workflow and trunk protection. An env var bypass that the guarded process itself can set reduces all guard checks to advisory — the agent can opt out of its own constraints.
