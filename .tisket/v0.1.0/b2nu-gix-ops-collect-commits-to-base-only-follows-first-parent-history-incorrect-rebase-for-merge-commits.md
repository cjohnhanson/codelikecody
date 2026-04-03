---
title: "gix_ops::collect_commits_to_base only follows first-parent history — incorrect rebase for merge commits"
status: todo
priority:
assignee:
labels: [clc, correctness, blocking, standard]
depends_on: []
created: 2026-03-23T03:11:53Z
updated: "2026-04-03T18:33:27Z"
---

## Problem

`collect_commits_to_base` (line 728 of `clc/src/gix_ops.rs`) should walk all commits on a branch back to the merge base so they can be replayed during rebase. Instead, it follows only the first parent at each step (`commit.parent_ids().next()`), meaning any merge commit on the branch causes the walk to skip the second-parent lineage entirely. The skipped commits are silently dropped from the replay, producing a rebase result that loses work introduced via the merged branch.

## Open Questions

- Does the current clc workflow ever produce merge commits on worker branches, or is this only reachable via manual intervention?
- Should the fix linearize the merge commit's changes into a single synthetic commit, or should it refuse to rebase branches containing merges?
- Is `find_merge_base` (which does use a full BFS walk) correct, or does it have the same first-parent bias?

## Why It Matters

Silent data loss during rebase. If a worker branch contains a merge commit — even an accidental one — the rebase will silently drop all commits reachable only through the second parent. The result looks clean (no error), but the work is gone.
