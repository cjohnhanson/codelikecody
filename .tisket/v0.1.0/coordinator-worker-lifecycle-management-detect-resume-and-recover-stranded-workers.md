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
