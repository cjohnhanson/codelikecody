---
title: "clc dispatch should clean up stale worktrees from prior failed runs before dispatching"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-02T04:21:00Z
updated: "2026-03-03T02:09:30Z"
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

## Scratch Notes

### Test Design
- Two missouri transitions added from `worker-stopped` → `redispatched-after-stale-cleanup`
  1. "re-dispatch cleans up stale worktree with dead PID" — PID file exists, process dead
  2. "re-dispatch cleans up stale worktree with no PID file" — PID file removed before dispatch
- Both save old PID to `/tmp/missouri-stale-old-pid` for comparison
- Target state `redispatched-after-stale-cleanup` verifies:
  - Fresh worktree + branch created
  - New worker alive with different PID
  - Tisket in `in_progress`
  - Still on main
  - Worker infrastructure (pipe, stdout) exists

### Key Implementation Notes
- `is_pickable()` only matches Todo/Blocked/Paused — NOT InProgress
- After first dispatch, tisket is `in_progress` → pickup would reject re-dispatch
- Dispatch cleanup must handle: remove worktree dir, prune git worktree metadata, delete branch
- Tisket status needs reset to `todo` (or pickup needs to tolerate in_progress for re-dispatch)
- Relevant code: `dispatch.rs` lines 46-58, `gix_ops.rs` (remove_worktree, delete_branch)
- `gix_ops::remove_worktree()` and `gix_ops::delete_branch()` already exist

### Files Created/Modified
- Created: `clc/tests/missouri/redispatched-after-stale-cleanup/` (state directory with assertions)
- Modified: `clc/tests/missouri/worker-stopped/.missouri/missouri.yml` (added 2 transitions)
