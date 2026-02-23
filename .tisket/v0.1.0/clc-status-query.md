---
title: "clc status query"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init, git-state-detection]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

`clc status` shows the current state: branch, phase, active tisket, missouri
test status. Useful as a debugging/inspection tool and as a building block for
hooks that need to read state.

## Missouri tests

States with various `.clc/state` contents:
- initialized-main: on main, no active work
- worktree-tests-unwritten: in worktree, phase is tests-unwritten
- worktree-implementing: in worktree, phase is implementing

Assertions:
- `clc status` output includes branch name
- `clc status` output includes current phase
- `clc status` output includes active tisket ID (if any)
- `clc status` exits 0 in all valid states
