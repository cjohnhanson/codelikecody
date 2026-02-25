---
title: "Status transitions"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init, clc-status-query]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

`clc status set <phase>` explicitly advances the work phase. The agent must call
this to move forward — the phase doesn't change automatically. This is the
mechanism that enforces the workflow sequence.

Phases (tentative): tests-unwritten → tests-written → red → implementing → green

## Missouri tests

State: worktree-tests-unwritten (`.clc/state` phase=tests-unwritten)
Transition: `clc status set tests-written` → worktree-tests-written
State: worktree-tests-written (`.clc/state` phase=tests-written)

Assertions:
- `.clc/state` file updated with new phase
- Invalid transitions rejected (e.g., can't go from tests-unwritten to green)
- `clc status set` with unknown phase name fails
- `clc status` reflects the new phase after transition
