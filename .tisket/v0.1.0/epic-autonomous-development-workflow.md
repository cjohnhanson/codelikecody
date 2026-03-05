---
title: "Epic: autonomous development workflow"
status: done
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:49Z
updated: "2026-03-05T04:17:54Z"
---

## Vision

Three-tier autonomous development: admin (human) → coordinators → workers.

**Admin session** — A human works with a Claude Code session on main. The admin manages tiskets (scoping discovery → todo), reviews landed work, and occasionally merges coordinator branches into main. The admin also handles permission escalations that coordinators can't resolve.

**Coordinator sessions** — One or more coordinators run in their own branches (not main). Each coordinator autonomously grabs todo tiskets matching its filter (label, dependency chain, project), dispatches workers, monitors them, lands their work into its integration branch, and loops. Coordinators are managed like workers — send/check/stop — but from the admin session.

**Worker sessions** — Workers branch off their coordinator's branch, implement a single tisket through the TDD phase cycle, and signal done. Workers escalate permission requests to their coordinator, which either grants (within its guidelines) or escalates to admin.

## Workflow

1. Admin scopes tiskets, marks them todo
2. Coordinator picks up todo tiskets matching its filter, dispatches workers
3. Workers implement (tests → red → implement → green → done)
4. Coordinator lands completed workers into its integration branch (squash per worker or batch)
5. Coordinator validates integration branch (cargo test + missouri run)
6. Admin reviews coordinator's branch, lands it into main (squash-merge)
7. Loop

## Component tiskets

### Foundation (must land first)
- `git-workflow-ephemeral-integration-branch-with-squash-merge-landing` — coordinator gets its own branch, workers branch off it, squash-merge landing
- `clc-land-should-rebase-branch-onto-head-before-fast-forward-merge` — auto-rebase before landing (used at every merge point)
- `clc-dispatch-should-clean-up-stale-worktrees-from-prior-failed-runs-before-dispatching` — cleanup before dispatch
- `coordinator-worker-lifecycle-management-detect-resume-and-recover-stranded-workers` — detect and recover stranded workers

### Coordinator improvements
- `coordinator-filtering-dispatch-by-label-dependency-chain-or-project` — filter what a coordinator works on
- `coordinator-merge-management-clearer-prompting-and-workflow-for-landing-worker-branches` — landing workflow prompts
- `multiple-concurrent-coordinators-with-tisket-claim-registry` — multiple coordinators, no double-dispatch

### Permission system
- `permission-escalation-chain-worker-to-coordinator-to-admin` — three-tier escalation
- `coordinator-permission-guidelines-configurable-auto-grant-policy-per-coordinator` — what coordinators can auto-grant

### Admin layer
- `admin-session-manage-coordinators-like-coordinators-manage-workers` — admin CLI for managing coordinators

### Quality gates
- `review-phase-coordinator-gated-code-review-before-merge` — code review before landing

## What already works
- Worker dispatch, phases, worktrees, send/check/stop/resume
- Permission request/grant/escalate/inbox
- Config-level permission allow rules
- Coordinator process (dispatch, monitor, land) — but on main, not its own branch
- `clc land` (fast-forward only)
