---
title: "SessionStart should auto-set initial phase on unphased feature branches"
status: backlog
priority: 2
assignee:
labels: [clc, hooks]
depends_on: []
created: "2026-02-26T04:13:10Z"
updated: "2026-02-26T04:13:10Z"
---

When SessionStart fires and detects: feature branch + tisket match + no phase set,
it should set the phase to `tests-unwritten` automatically. This bootstraps
manually-created worktrees into the phase system on first agent session.

Currently only `clc pickup` sets the initial phase (line 84 of `pickup.rs`).
Any worktree created outside that path — manual `git worktree add`, Claude Code's
worktree mode, plain branching — starts with no phase and no enforcement.

The hook already has access to git state, tisket state, and phase. The detection
logic is straightforward. The phase write uses `crate::phase::set`.
