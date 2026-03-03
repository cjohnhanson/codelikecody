---
title: "Admin session: manage coordinators like coordinators manage workers"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:41Z
updated: "2026-03-03T04:15:39Z"
---

## Problem

Coordinators are currently fire-and-forget processes on main. There's no CLI for managing them the way coordinators manage workers. The admin (human) needs the same send/check/stop/land interface for coordinators that coordinators have for workers.

## Design

- `clc coordinators` — list running coordinators with status, branch, worker count, last activity
- `clc coordinator <id> check` — recent output from a coordinator
- `clc coordinator <id> send "<message>"` — send a message to a coordinator
- `clc coordinator <id> stop` — stop a coordinator
- `clc coordinator <id> land` — squash-merge the coordinator's integration branch into main
- `clc coordinator <id> log` — full output log

Coordinators register themselves in `.clc/coordinators/` on main (or a shared location) so the admin session can discover them. Each coordinator has: pid, branch name, worktree path, stdin pipe, stdout log.

## Depends on
- `git-workflow-ephemeral-integration-branch-with-squash-merge-landing` (coordinators need their own branches first)
