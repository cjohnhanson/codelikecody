---
title: "clc abandon — escape hatch for incomplete work"
status: discovery
priority:
assignee:
labels: [feature, clc]
depends_on: []
created: "2026-02-26T03:16:13Z"
updated: "2026-02-26T03:16:13Z"
---

Without an escape hatch, the stop hook can trap an agent in an incomplete
phase with no way out. `clc abandon` provides a clean exit.

## Behavior

- Clears `.clc/state` (removes phase)
- Sets tisket status to `blocked` (or whatever the enum settles on)
- Leaves worktree and branch intact — work may be resumable later
- Unblocks the stop hook so the session can end

## Prerequisites

- Depends on the fixed status enum tisket for the abandon target status

## Guard protection

Once abandon exists, also guard `.clc/state` from direct manipulation:
- Block Bash commands that reference `.clc/state` (rm, echo >, cat >, etc.)
- Block Edit/Write targeting `.clc/state`
- Phase transitions should only happen through `clc status set` and `clc abandon`
