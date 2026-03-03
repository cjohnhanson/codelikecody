---
title: "Coordinator worker lifecycle management: detect, resume, and recover stranded workers"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T00:56:37Z
updated: "2026-03-03T02:09:06Z"
---

## Problem

Workers can end up stranded — they did real work but died before completing the phase ceremony (green → done → land). A new coordinator has no awareness of prior workers or their state. It dispatches fresh, hits stale worktree/branch conflicts, and the completed work is effectively invisible.

Current gaps:

- Coordinator only knows about workers it spawned in the current session
- No persistent registry of workers across coordinator sessions
- No way to detect a worktree with committed work that just needs phase advancement
- `clc workers` only lists workers with directories in `.clc/worker/`, which is coordinator-local
- Resume (`clc worker <id> resume`) exists but requires knowing the worker exists

## Design considerations

- Worker registry: persistent state (file or git-tracked) that records active workers, their worktree paths, branch names, and last known phase
- Coordinator startup: scan registry, check which workers are alive (PID check), which have committed work, which are truly dead
- Recovery actions: for workers with committed work at `implementing` or later — advance phases and finalize without re-dispatching
- Stale cleanup: for workers that died before committing anything — clean up worktree and branch, re-dispatch
- Relationship to git workflow tisket: the integration branch model changes where workers live, but the lifecycle problem remains the same

## Scratch Notes

### Session 1 — Test Design Phase

**Files consulted:**
- `clc/src/worker.rs` — collect_workers, prune, resume, supervise, land
- `clc/src/coordinate.rs` — coordinator launch, pickable tiskets, coordinator prompt
- `clc/src/dispatch.rs` — spawn_worker_process
- `clc/src/done.rs` — finalization (green to done + tisket close + commit)
- `clc/src/phase.rs` — phase load/set/transitions
- `clc/src/cli.rs` — CLI structure (Workers, Worker, Land, etc.)
- `clc/src/main.rs` — command routing
- `clc/tests/missouri/` — all existing test states for patterns

**Current gaps identified:**
1. collect_workers() only finds workers with .clc/worker/ dirs — after prune, worker disappears
2. No scan of worktrees for phase state independent of worker process state
3. No concept of "stranded" — just "alive" or "dead"
4. No recovery path that does phase advancement + done + land without spawning new claude process
5. No persistent registry across coordinator sessions

**Test design — new commands to test:**
1. `clc workers --stranded` — lists workers where: process dead, branch exists, phase >= implementing, committed work
2. `clc worker <id> recover` — for stranded worker at green: run done on worktree, allow land from trunk

**State graph:**
- New root: `stranded-at-green` — simulates worker that reached green phase with committed work, then died
- Tests `clc workers --stranded` detection
- Tests `clc worker <id> recover` to finalize
- Tests recovery of worker at earlier phases (should fail or require different handling)
- Tests that recovering alive worker fails
