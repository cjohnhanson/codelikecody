---
title: "Git workflow: ephemeral integration branch with squash-merge landing"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T00:48:45Z
updated: "2026-03-03T01:35:54Z"
---

## Problem

The coordinator currently works directly on main. Workers branch off main independently. When workers complete, landing requires fast-forward merging each branch onto main — but main advances as workers land, breaking subsequent fast-forwards. The coordinator has no integration surface of its own.

## Design

Adopt the integration manager pattern (Pro Git) with git.git-style throwaway integration branches.

### Batch lifecycle

1. `clc coordinate` creates an ephemeral integration branch off current main HEAD (e.g., `integrate/<timestamp>` or `integrate/<batch-id>`)
2. Workers branch off the integration branch, work in worktrees as they do now
3. As workers reach `done`, coordinator merges each worker branch into the integration branch
4. If a merge conflicts: skip that worker, flag it, re-dispatch, or attempt resolution
5. Once all workers in the batch are merged and validated (cargo test + missouri run), the integration branch squash-merges onto main
6. Integration branch and worker branches are deleted

### Squash-merge landing

The integration branch lands on main as a single squash commit. Individual worker commits are not preserved on main — the squash commit message should summarize what was included (list of tiskets landed, one-line per worker). This keeps main history clean and makes bisection meaningful at the batch level rather than the individual-agent-edit level.

### What if main advances during a batch?

Rebuild the integration branch: create a new one from current main, re-merge completed worker branches onto it. The throwaway nature means this is always safe — no state to preserve on the integration branch itself.

### Coordinator's own commits

The coordinator should not commit directly to the integration branch. Tisket management (status changes, notes) happens on main before or after the batch. The integration branch is purely a merge target for worker branches.

### Conflict handling

- Try octopus merge first when multiple workers complete simultaneously (fast path for non-overlapping changes)
- Fall back to sequential merge-per-worker if octopus fails
- Any single worker merge conflict: skip that worker, report to user, continue with remaining
- Skipped workers can be re-dispatched in the next batch after the conflict source is landed

### Implementation scope

- New `integrate` module in clc for branch lifecycle (create, merge-worker, validate, squash-land, cleanup)
- Update `clc coordinate` to create integration branch instead of working on main
- Update `clc dispatch` to branch workers off integration branch (or accept a base branch)
- Update `clc land` to merge into integration branch, not main
- New `clc land --batch` or `clc integrate land` to squash-merge integration branch onto main
- gix operations: octopus merge (stretch), sequential merge, squash commit

### Prior art

- git.git `pu` branch (throwaway integration rebuilt from scratch)
- GitLab merge trains / GitHub merge queues (batch + validate + land)
- Integration manager workflow (Pro Git ch5)
