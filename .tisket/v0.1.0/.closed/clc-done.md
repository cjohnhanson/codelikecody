---
title: "clc done"
status: done
priority:
assignee:
labels: [feature]
depends_on: [missouri-integration, tisket-integration, status-transitions]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

`clc done` marks work complete:
1. Run missouri tests, verify all green
2. Set phase to "done" in `.clc/state`
3. Close the tisket

The stop hook uses this — agent can't stop until `clc done` succeeds.

## Missouri tests

State: worktree-green (all missouri tests passing, phase=green)
Transition: `clc done` → completed
State: completed (`.clc/state` phase=done, tisket closed)

Assertions:
- Tisket status is now closed/done
- `.clc/state` phase is "done"
- `clc done` fails if missouri tests are not all green
- `clc done` fails if not in a worktree
