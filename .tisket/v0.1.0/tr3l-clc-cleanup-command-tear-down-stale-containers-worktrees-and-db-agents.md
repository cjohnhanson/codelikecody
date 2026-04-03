---
title: "clc cleanup command — tear down stale containers, worktrees, and DB agents"
status: todo
priority: 2
assignee:
labels: [clc, auto]
depends_on: []
created: "2026-04-03T18:27:09Z"
updated: "2026-04-03T18:27:09Z"
---

## Problem

After `clc up` is killed or crashes, Docker containers, git worktrees, and
coordination DB entries are left behind in stale states. There's no single
command to clean up. Currently requires manual `docker stop`, `git worktree
remove`, and DB manipulation. This makes iteration on `clc up` painful —
every restart accumulates garbage.

## Proposed solution

`clc cleanup` command that:

1. Stops and removes Docker containers created by clc (identified by label
   or naming convention)
2. Removes git worktrees under `.worktrees/` that have no live worker process
3. Resets non-terminal agents (Pending, Running) in the coordination DB to
   Stopped
4. Removes stale `.clc/` runtime state (CA certs, coordination DB) so the
   next `clc up` starts fresh

Should be safe to run at any time. Should not touch the admin worktree or
worktrees with live processes.

## Done When

- `clc cleanup` stops all clc Docker containers (not unrelated containers)
- `clc cleanup` removes worktrees with no live worker PID
- `clc cleanup` resets Pending/Running agents to Stopped in the DB
- `clc cleanup --dry-run` prints what would be cleaned without acting
- Running `clc up` after `clc cleanup` starts clean with no stale state
- At least one test verifies dry-run output
- At least one test verifies container cleanup doesn't touch non-clc containers
