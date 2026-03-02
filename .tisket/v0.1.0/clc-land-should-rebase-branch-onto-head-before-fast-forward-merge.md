---
title: "clc land should rebase branch onto HEAD before fast-forward merge"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-02T03:56:22Z"
updated: "2026-03-02T03:56:22Z"
---

`clc land <id>` currently requires the worker branch to be a direct descendant of HEAD (fast-forward only). When main advances during a coordinator run — from tisket status updates, other workers landing, or manual commits — the branch falls behind and `clc land` fails.

The coordinator has no way to fix this: `git rebase` is blocked by the trunk allowlist, and workers can't rebase either (Claude Code permission system blocks it).

## Fix

`clc land` should automatically rebase the worker branch onto HEAD before attempting the fast-forward merge. This is safe because:
- Worker branches are single-owner (no one else is committing to them)
- The rebase happens inside the worktree, not on trunk
- If the rebase has conflicts, `clc land` can fail with a clear error

## Implementation

In `merge.rs` (or wherever `clc land` lives):
1. Before the fast-forward check, detect if the branch is behind HEAD
2. If behind, rebase the branch onto HEAD using gix (NOT by shelling out to git)
3. If rebase succeeds, proceed with fast-forward
4. If rebase has conflicts, abort and report the conflict

## Observed in

Coordinator run on 2026-03-02: scratch-notes worker completed successfully but couldn't land because main had advanced with tisket scoping commits. Coordinator got stuck, resumed the worker to try rebasing, worker couldn't rebase either. Required manual intervention.
