---
title: "Phase and workflow state lives in flat files instead of the coordination database"
status: in_progress
priority: 3
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-24T00:53:21Z
updated: "2026-03-24T01:06:20Z"
---

## Problem

Phase state is the source of truth for what a worker is doing and what
it's allowed to do. It lives in `.clc/state` — a flat file in each
worktree with a line like `phase: implementing`. The coordination
database already exists, already tracks agents, already receives phase
transition notifications via `StatusUpdate` messages in `phase::set()`.
But nothing reads the DB back for decisions — it's write-only for phase
data.

The coordination DB and the state file can drift. The hook system reads
the file (`phase::load(cwd)` in `hook.rs:39`). The coordinator reads
the file. `done.rs` reads the file. `worker.rs` stranded/recover logic
reads the file. The DB gets a copy via `StatusUpdate` messages but
nobody trusts it. Two sources of truth for the same data, one
authoritative and fragile (a flat file that any stray write could
corrupt), one durable but ignored.

The state file also can't carry workflow context. It stores
`phase: implementing` but not *which workflow* the phase belongs to.
If different workflows define different phase names and guard policies
(see 86or), the guard needs to know the active workflow — and the file
doesn't have it. Adding a `workflow:` field to the flat file is possible
but deepens the wrong abstraction.

Other file-based state in `.clc/` has the same problem:
`permission-request.json` (worker permission requests), worker PID
files, cursor files for coordinator output tracking. The coordination
DB was designed to replace all of this, and partially does (it has
`PermissionRequest`/`PermissionGrant` message kinds, agent status
tracking, PID storage via `set_pid`), but the file-based paths are
still the primary mechanism in several code paths.

## Open Questions

- The hook runs with `cwd` set to the worktree (`.worktrees/<id>/`).
  The DB lives at the project root. The hook would need to resolve the
  project root to query the DB. The parent-walking pattern already
  exists in `is_worker_alive` and `send_prompt` — is that acceptable,
  or should the hook receive the project root explicitly (e.g., via an
  env var set during worktree init)?
- What about the admin worktree, which has no phase and no DB entry?
  Does it need a row, or does "no row = no enforcement" suffice?
- Should the state file be removed entirely, or kept as a fallback for
  bare worktrees without coordination (e.g., manual `clc pickup` in
  a project that hasn't run `clc init` with coordination)?
- What other file-based state in `.clc/` should move at the same time?
  Candidates: `permission-request.json`, worker PIDs, coordinator
  cursors. Moving everything at once is cleaner but bigger. Moving
  just phase state is smaller but leaves the mixed-state problem
  partially unsolved.
- The DB currently uses SQLite (local file). There's also a Postgres
  backend in progress (see `io9i` branch). Should this migration target
  the `CoordinationBackend` trait so it works with both backends, or
  is SQLite-first acceptable?

## Why It Matters

Dynamic workflows (86or) need the guard to know both the current phase
and the active workflow at hook evaluation time. The flat file can't
carry workflow identity without becoming a second config store. The DB
already has the context — the agent registration knows the tisket ID,
the tisket has labels, labels resolve to a workflow via
`config::resolve_workflow`. Moving state to the DB makes the dynamic
workflows feature natural instead of forced.

More generally, the flat file approach doesn't scale to multi-process
coordination. When a coordinator, multiple workers, and a supervisor
are all running, the DB is the only reliable way to share state. The
files work for single-agent interactive use but become a liability
under autonomous operation.

## Scratch Notes
