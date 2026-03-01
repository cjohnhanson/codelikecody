---
title: "Admin and tisket operations should never dirty main"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T20:46:16Z
updated: "2026-03-01T02:56:56Z"
---

## Problem

Several coordinator operations modify files on main as a side effect, leaving the trunk dirty. This violates the principle that main should always be clean and ready for branching.

## Observed dirty-main scenarios

1. **`clc done` on worktree deletes tisket file** — The `tisket issue close` inside `clc done` deletes the tisket markdown from `.tisket/`, but this shows as a deletion on main's view of the file. When landing, the coordinator has to manually close the tisket on main, commit it, rebase the branch, then land.

2. **`clc land` requires pre-closing tisket on main** — Because `clc land` checks tisket status on main (not the branch), the coordinator has to close the tisket on main first, commit, rebase branch, then fast-forward. This creates administrative commits on main.

3. **Worktree operations leaking to main** — Unexpected diffs have appeared on main during parallel worker operations (missouri.rs, test files). Possibly git worktree race conditions, possibly worker operations touching shared state.

## Principle

Main should be a clean sequence of: pickup commits, feature commits, finalize commits. No administrative fixup commits. No manual tisket closures on main. The `clc land` workflow should handle everything atomically.

## Potential approach

- `clc land` should handle tisket closure as part of the merge, not require it as a precondition
- `clc done` on a worktree should mark the tisket as done in a way that's branch-local, not modifying the file that main also sees
- Or: tisket status lives in branch-local state (`.clc/`) rather than in the tisket file itself
