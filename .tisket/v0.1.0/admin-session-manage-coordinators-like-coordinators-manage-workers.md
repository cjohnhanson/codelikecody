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

## Scratch Notes

### Test Design (Missouri state graph)

**Two test paths from `initialized`:**

1. **coordinator-registered → coordinator-stopped** — Tests admin CLI for managing a live coordinator
   - Transition manually creates `.clc/coordinators/test-coord/` with pid, stdout.jsonl, stdin.pipe
   - Uses `sleep 3600` as a mock coordinator process
   - Background `cat` keeps pipe read-end open so `send` doesn't hang
   - Tests: `clc coordinators` (list), `clc coordinator <id> check/log/send/stop`
   - coordinator-stopped verifies: dead process, `--all` flag, log still works, send fails

2. **coordinator-with-integration → coordinator-landed** — Tests coordinator landing
   - Creates `integrate/test-coord` branch with a commit
   - Registers dead coordinator with branch reference in `.clc/coordinators/test-coord/branch`
   - Tests: `clc coordinator <id> land` squash-merges integration branch to main
   - coordinator-landed verifies: branch deleted, content on main, registration cleaned up

### Registration State Layout

```
.clc/coordinators/<id>/
├── pid           — process ID
├── stdout.jsonl  — NDJSON output log
├── stdin.pipe    — named FIFO for messages
├── branch        — integration branch name (e.g., integrate/<id>)
└── stderr.log    — error log (optional)
```

### CLI Commands to Implement

```
clc coordinators [--all]           — list coordinators
clc coordinator <id> check         — cursor-based output
clc coordinator <id> log [--lines] — parsed log
clc coordinator <id> send "msg"    — message via pipe
clc coordinator <id> stop          — SIGTERM/SIGKILL
clc coordinator <id> land          — squash-merge integration branch
```

### Key Decisions
- Coordinators register in `.clc/coordinators/<id>/` (NOT `.clc/worker/`)
- Registration is separate from the old COORDINATOR_ID="coordinator" special case in worker.rs
- The new coordinator_mgmt module mirrors worker.rs but reads from `.clc/coordinators/`
- `land` uses existing integrate module (squash-merge to main)
- Rust unit tests deferred to implementation phase (phase guards block src/ writes during test phase)

### Files Consulted
- `clc/src/worker.rs` — pattern for list/check/log/send/stop/land (the model to mirror)
- `clc/src/coordinate.rs` — current coordinator launch, spawn_worker_process
- `clc/src/cli.rs` — clap command definitions
- `clc/src/main.rs` — command routing
- `clc/src/integrate.rs` — integration branch operations (create/merge/land)
- `clc/src/dispatch.rs` — spawn_worker_process, worker state setup
- Existing missouri tests: claim-ready, dispatched, worker-stopped (patterns followed)
