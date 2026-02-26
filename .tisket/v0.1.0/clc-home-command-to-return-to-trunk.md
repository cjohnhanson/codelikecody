---
title: "clc home command to return to trunk"
status: backlog
priority: 2
assignee:
labels: [clc]
depends_on: []
created: "2026-02-26T04:19:51Z"
updated: "2026-02-26T04:19:51Z"
---

After `clc done`, there's no command to get back to trunk. The agent is stuck
on a completed feature branch with nothing to do. `clc home` returns to the
main worktree.

Behavior:
- Changes cwd to the repository root (not the worktree)
- Verifies no uncommitted changes on the current branch before leaving
- Works from any branch/worktree — feature, admin, or nested
- If already on trunk, no-op

This is the bridge between `clc done` and `clc merge`. The workflow loop:
`clc pickup` → work → `clc done` → `clc home` → `clc merge <id>`
