---
title: "clc dispatch should clean up stale worktrees from prior failed runs before dispatching"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-02T04:21:00Z
updated: "2026-03-03T02:09:06Z"
---

When `clc dispatch <id>` finds a pre-existing worktree and branch from a prior failed run, it fails. The coordinator can't clean these up because `git branch -D` and worktree removal are blocked on trunk.

## Expected behavior

`clc dispatch` should detect stale worktrees (no live worker process) and clean them up automatically before creating a fresh worktree and dispatching. This makes dispatch idempotent — a prior crash or failed run doesn't permanently block re-dispatch.

## Detection

A worktree is stale if:
- `.clc/workers/<id>/pid` doesn't exist, OR
- The PID in the file is not alive

## Cleanup

1. Remove the worktree directory (`.worktrees/<id>/`)
2. `git worktree prune`
3. Delete the branch (`refs/heads/<id>`)
4. Proceed with normal dispatch (create fresh worktree + branch)

## Observed in

Coordinator run on 2026-03-02: `clc dispatch refactor-missouri-sandbox-into-a-proper-backend-trait` failed because a stale worktree existed from a prior coordinator run. Required manual intervention (`rm -rf` + `git worktree prune` + `git branch -D`).
