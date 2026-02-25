---
title: "Attempt gating for status transitions"
status: done
priority:
assignee:
labels: [feature]
depends_on: [status-transitions, clc-config]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

Configurable gate: a status transition must be attempted N times before it
succeeds. First N-1 attempts get a "double check / are you sure" prompt. Nth
attempt succeeds. The count is tracked in `.clc/state`. Configurable per
transition via clc config.

This forces the agent to reconsider before advancing, and gives hooks another
opportunity to evaluate the work.

## Missouri tests

State: worktree-tests-written (attempt_count=0, required_attempts=3)
Transition: `clc status set implementing` → still tests-written (attempt 1/3)
Transition: `clc status set implementing` → still tests-written (attempt 2/3)
Transition: `clc status set implementing` → worktree-implementing (attempt 3/3)

Assertions:
- First N-1 attempts: exit non-zero, `.clc/state` phase unchanged, attempt count incremented
- Nth attempt: exit 0, phase advanced, attempt count reset
- Attempt count persists in `.clc/state`
