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

---

## Investigation update (2026-02-28)

### Stop hook investigation (failures #2 and #5)

**Previous hypothesis was wrong.** Stop events DO fire in --print mode. Claude Code docs say only PermissionRequest hooks are disabled in non-interactive mode. The Stop hook is registered in the worker's settings.local.json and fires normally.

**Actual root cause for #5 (green-phase stop):** check_stop() in guard.rs line 72 returns Passthrough for Green phase: `Some(Phase::Done | Phase::Green) => return Response::Passthrough`. The hook fires, evaluates the phase, and allows the stop. Fix: remove Green from passthrough arm so only Done permits stopping.

**#2 (implementing-phase stop) is still unexplained.** check_stop() should block implementing. Needs investigation with logging to determine whether the hook errored, the model bypassed it somehow, or something else.

Dedicated fix tisket: `stop-event-does-not-fire-in-print-mode-workers-exit-before-done-phase` (title is now outdated — actual issue is check_stop allowing green)

### Prime text missing operational instructions (failure #3)

The prime text describes missouri state but doesn't say how to run tests (`clc missouri run`). This is a content problem, not a compaction problem. Compaction makes it worse but the prime never teaches it in the first place.

Dedicated fix tisket: `context-compaction-drops-operational-knowledge-workers-forget-project-tooling` (reframed as prime text content issue)


### 6. Worker hacked missouri PATH instead of using clc missouri run
- **Worker**: stop-event-does-not-fire-in-print-mode (first dispatch)
- **What happened**: Worker needed to run missouri tests against its local build.
  Instead of using `clc missouri run`, rewrote the missouri.yml PATH to point at
  the worktree's target/debug, ran tests, then reverted the change.
- **Root cause**: Two factors. (a) The prime text said `missouri run` not
  `clc missouri run` -- the exact bug the other worker was fixing simultaneously.
  (b) Parallel workers dispatch from the same commit, so worker A can't benefit
  from worker B's fix even when B fixes the exact problem A encounters.
- **Status**: (a) Fixed by context-compaction worker. (b) Open -- sequential
  dispatch or mid-flight rebase would help.

### 7. Worker stopped at implementing despite stop hook being configured
- **Worker**: stop-event-does-not-fire-in-print-mode (first dispatch)
- **What happened**: Worker emitted result:success at implementing phase after
  89 turns. Got tests green but never advanced phase or ran clc done. No Stop
  event visible in the NDJSON output -- the result message follows immediately
  after the last tool result with no intervening stop event.
- **Root cause**: Strong evidence that Stop events do not fire in --print mode.
  check_stop() would block implementing, but the Stop event never appears in
  the output stream. The context-compaction worker reached done by choice
  (ran clc done itself), not because any hook forced it.
- **Status**: Open. The request-review phase concept may be the right solution
  rather than relying on a stop hook that doesn't fire.

### 8. Worker reached done without committing implementation code
- **Worker**: context-compaction-drops-operational-knowledge-workers-forget-project-tooling
- **What happened**: Worker fixed 3 occurrences of `missouri run` → `clc missouri run`
  in prime text, reached done phase, closed tisket. But the code changes were never
  committed. On `clc land`, only the finalize commit (tisket closure) merged. The
  actual fix was lost entirely.
- **Root cause**: `clc done` doesn't verify the working tree is clean. Worker ran
  `clc done` without having staged/committed its implementation. Phase advanced,
  tisket closed, worktree removed — code gone.
- **Fix**: `clc done` should refuse to finalize with uncommitted changes.
- **Status**: Open — tisket `clc-done-must-verify-clean-working-tree-before-finalizing`

### 9. Landing branches requires manual tisket closure on main
- **What happened**: `clc land` checks tisket status on main, not the branch. Since
  `clc done` on the worktree only modifies the branch-local copy of the tisket file,
  main still shows `in_progress`. Coordinator must: close tisket on main → commit →
  rebase branch → land. Creates administrative commits.
- **Root cause**: Tisket status is stored in the tisket markdown file, which is shared
  across all branches. `clc done` commits the closure on the branch, but `clc land`
  reads the file on main.
- **Status**: Open — tisket `admin-and-tisket-operations-should-never-dirty-main`
