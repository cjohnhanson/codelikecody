---
title: "integrate.rs merge/land logic has zero tests — complex git graph operations uncovered"
status: todo
priority:
assignee:
labels: [clc, testing, blocking]
depends_on: []
created: "2026-03-23T03:12:04Z"
updated: "2026-03-23T03:12:04Z"
---

## Problem

1. `clc/src/integrate.rs` (416 lines) implements three-way tree merges (`merge`), squash-land-onto-main (`land`), and integration branch creation (`create`) using raw `gix` operations — merge base discovery, ancestor walking, tree checkout, branch ref manipulation, and worktree index updates. These git-graph operations should have tests covering at minimum: successful merge, conflict detection, already-merged detection, land with multiple merged branches, and the branch cleanup path.
2. There are zero `#[test]` attributes and no `#[cfg(test)]` module in `integrate.rs`. The hand-rolled `is_ancestor`, `find_merge_base`, and `collect_merged_branches` functions perform graph traversals with no verification. `land` deletes branch refs and rewrites HEAD — destructive operations with no safety net.
3. A bug in merge-base discovery or ancestor detection could silently produce incorrect merges or allow landing when the integration branch has diverged from main. The `collect_merged_branches` function follows only first-parent history, which may miss branches in certain merge topologies — but there's no test to confirm or deny this.

## Open Questions

- Can `gix`-based operations be tested against an in-memory or temp-dir git repo, or do they require a real filesystem repo with actual commits?
- Is the first-parent-only walk in `collect_merged_branches` intentional, or is it a bug that would surface with octopus merges or rebased integration branches?
- Should `update_worktree` (which does a full checkout from a tree ID) be tested separately, given it touches the filesystem and index?

## Why It Matters

`integrate` is the path that lands all worker output onto main. A merge bug means corrupted code on trunk. A land bug means lost work or orphaned branches. These are the highest-consequence git operations in the system, and they have zero automated verification.
