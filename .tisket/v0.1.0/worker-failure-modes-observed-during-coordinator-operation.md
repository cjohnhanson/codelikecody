---
title: "Worker failure modes observed during coordinator operation"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-02-28T19:35:08Z"
updated: "2026-02-28T19:35:08Z"
---

Running log of failure modes observed in dispatched workers. Each entry
captures what happened, root cause if known, and whether it's been addressed.

## Observed Failures

### 1. Budget cap killed worker mid-task
- **Worker**: configurable-per-transition-phase-gates (first dispatch)
- **What happened**: Worker hit $5 hardcoded budget ceiling at 79 turns, died
  at tests-written phase before starting implementation
- **Root cause**: `clc dispatch` had `--max-budget-usd 5` hardcoded as default
- **Fix**: Removed budget cap entirely (Max subscription, no marginal cost)
- **Status**: Fixed

### 2. Worker stopped at implementing without reaching done
- **Worker**: configurable-per-transition-phase-gates (second dispatch)
- **What happened**: Worker emitted `result: success` after 134 turns but was
  still at `implementing` phase. Never ran final tests or advanced to green/done.
- **Root cause**: Unknown. The Stop hook should reject premature stops but
  the worker somehow exited cleanly. Possibly the claude process itself decided
  to stop (model-level decision vs hook enforcement).
- **Fix**: None yet. `clc worker resume` now exists as mitigation.
- **Status**: Open — need to investigate Stop hook enforcement

### 3. Context compaction loses project-specific knowledge
- **Worker**: configurable-per-transition-phase-gates (resumed session)
- **What happened**: After context compaction, worker forgot how missouri
  tests are run (tried running `missouri run` directly, got PATH errors for
  clc/tisket/cargo). The missouri tests use wrapper scripts in `.missouri/bin/`
  that set up PATH correctly.
- **Root cause**: Context compaction drops details about project tooling
  and test infrastructure. The SessionStart hook re-injects phase and workflow
  context but not operational details like "use cd clc/tests/missouri && missouri run".
- **Fix**: None yet. Could add missouri run instructions to SessionStart prime text.
- **Status**: Open

### 4. Dead workers accumulate in clc workers list
- **Worker**: All 5 from first dispatch batch
- **What happened**: Workers that completed successfully still show as "working"
  in `clc workers` because the process exited but the PID file remains.
- **Root cause**: No reaping/cleanup of completed workers.
- **Fix**: Separate tisket exists (worker-cleanup-prune-dead-workers)
- **Status**: Open — tisket exists

### 5. Worker stops at green saying "ready for clc done whenever you want"
- **Worker**: configurable-per-transition-phase-gates (resumed session)
- **What happened**: Worker reached green phase (12/12 missouri, all committed),
  then said "Ready for `clc done` whenever you want to finalize" and stopped.
  Treated `clc done` as something requiring human approval rather than running it.
- **Root cause**: Same as #2 — Stop hook isn't enforcing "must reach done phase."
  The system prompt says "Do not stop before reaching the 'done' phase" but the
  model treats it as advisory. The Stop hook should mechanically reject the stop.
- **Fix**: Need to investigate why Stop hook isn't firing or isn't blocking.
  The hook should check phase != done and return a blocking error.
- **Status**: Open — reproduced twice now, same pattern
