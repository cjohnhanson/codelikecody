---
title: "Permission escalation chain: worker to coordinator to admin"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:43Z
updated: "2026-03-03T04:38:35Z"
---

## Problem

Permission escalation currently goes worker → user directly. With coordinators as a middle layer, the chain should be worker → coordinator → admin. Coordinators should be able to handle routine permission requests (within their guidelines) without bothering the human.

## Design

### Three-tier chain

1. Worker hits a permission denial → files `clc permissions request "<description>"`
2. Coordinator sees the request (via `clc permissions list` or `clc worker <id> check`)
3. Coordinator decides:
   - **Grant** — if the request falls within its auto-grant policy → `clc permissions grant <worker-id> "<rule>"`
   - **Escalate** — if outside its policy → `clc permissions escalate <worker-id> "<description>"`
4. Escalation lands in `.clc/escalations/` visible to the admin session
5. Admin reviews via `clc permissions inbox`, grants or denies

### Changes from current system

The current escalate command writes to `.clc/escalations/` on the coordinator's working directory. For admin visibility, escalations need to be written somewhere the admin session can see — either on main (requires coordinator to commit to main, which we're trying to avoid) or in a shared location. Options:
- Coordinator pushes escalation files to its branch, admin reads from there
- Shared escalation directory outside of git
- Coordinator sends a message to admin session (if admin session has an inbound pipe)

## Depends on
- `coordinator-permission-guidelines-configurable-auto-grant-policy-per-coordinator`
- `admin-session-manage-coordinators-like-coordinators-manage-workers`

## Scratch Notes

### Analysis (2026-03-02)

**Current state**: Permission chain worker → coordinator → user exists but only works from trunk. The `cmd_permissions` function in main.rs uses `std::env::current_dir()` for all subcommands. When admin runs from `.worktrees/clc-admin/`, it can't see escalations on trunk.

**Root cause**: `permissions::inbox()`, `permissions::grant()`, `permissions::list()`, `permissions::escalate()` all receive `cwd` as `project_dir`. They need `home::home(cwd)` to resolve the project root from any worktree.

**Key files**:
- `clc/src/permissions.rs` — core logic
- `clc/src/cli.rs` — PermissionsAction enum (needs Deny variant)
- `clc/src/main.rs:315-330` — cmd_permissions routing (needs home resolution)
- `clc/src/home.rs` — project root resolution via gix

**What needs to change**:
1. `cmd_permissions` should use `home::home(&cwd)` for grant/list/escalate/inbox/deny
2. `request` still uses `cwd` (worker perspective)
3. New `Deny` subcommand: removes escalation, updates request to denied status
4. `PermissionRequest` struct needs `Denied` status variant and optional `denial_reason` field

**Test plan**: Three new Missouri states:
- `escalation-admin-visible` — admin worktree can see trunk escalations
- `escalation-admin-granted` — admin grants from worktree, resolves cross-worktree
- `escalation-admin-denied` — admin denies from worktree, updates request status

**Graph**: escalation-pending → (clc admin) → escalation-admin-visible → (grant) → escalation-admin-granted / (deny) → escalation-admin-denied
