---
title: "SQLiteBackend: replace filesystem communication with SQLite-backed CoordinationBackend"
status: in_progress
priority:
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-23T00:27:05Z
updated: "2026-03-23T00:27:14Z"
---

## Scratch Notes

### Done (2026-03-22)

**DbBackend** — unified SeaORM backend in `clc-sdk/src/coordination_db.rs`.
Works with both SQLite (in-memory or file) and Postgres. Auto-detects DB type
from connection URL. `sqlite` and `postgres` feature flags. 22 contract tests
pass against both backends.

**Sync wrapper** — `clc/src/coordination.rs`. `Coordination::open(project_dir)`
creates `.clc/coordination.db` via SQLite. Tokio current-thread runtime for
blocking on async trait methods.

**Wired into clc lifecycle:**
- dispatch: register_agent + set_status(Running) + set_pid
- stop: set_status(Stopped)
- resume: set_status(Running) + set_pid
- done: set_status(Completed)
- phase transitions: send(StatusUpdate) — only if DB exists
- permission request/grant/deny/escalate: send messages
- send_prompt: send(Text)
- recv_output: send(Output) summaries
- is_worker_alive: checks DB status
- list_workers: enriches filesystem scan with DB status
- check: shows coordination messages
- supervise: checks DB completion

All writes go to DB. All reads check DB first, fall back to filesystem.
Guard: `db_path.exists()` prevents surprise DB creation in test fixtures.

**36/36 missouri paths pass, 4447 assertions, zero warnings.**

### Not done

- `inbox` and `pending_request` reads from DB — attempted, caused 3 missouri
  failures (reverted). Need to investigate why the DB query path breaks the
  escalation test flow.
- `list_workers` fully from DB (not just enrichment) — blocked on output
  format migration for missouri assertions.
- Phase reads from DB — `phase::load` still reads `.clc/state` file.
- Cursor persistence in DB — still filesystem.
- Remove filesystem writes entirely — can't until reads are migrated.

### Architecture insight: coordinator is not a Claude process

The coordinator should be a poll loop over the coordination DB, not a
long-running Claude session. The loop:
1. Check DB for new permission requests → auto-grant / escalate / deny
2. Check DB for completed workers → land
3. Check DB for failed workers → resume or flag
4. Check DB for pickable tiskets → dispatch
5. Sleep(interval)

The LLM is a tool the coordinator calls for judgment, not the process itself.
This is the loop wrapper (f1dm). The coordination DB makes it possible.
