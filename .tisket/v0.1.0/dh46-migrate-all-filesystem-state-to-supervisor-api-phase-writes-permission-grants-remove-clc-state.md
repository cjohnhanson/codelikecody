---
title: "Migrate all filesystem state to supervisor API — phase writes, permission grants, remove .clc/state"
status: in_progress
priority:
assignee:
labels: [supervisor, architecture, clc-up-target]
depends_on: []
created: 2026-03-25T03:15:51Z
updated: "2026-03-25T03:16:29Z"
---

## Scratch Notes

### Status: tests written, phase advancing

**Done:**
- `set_via_api()` in phase.rs — reads current phase from API, validates transition, writes to API, no filesystem
- `load_phase_from_api()` in phase.rs — reads phase from API
- `load()` and `set()` both check CLC_API_URL and route through API when set
- 5 new integration tests in phase.rs: roundtrip, db storage, transition validation, load from API, default when unset
- Tests use in-memory SQLite + plain HTTP API server (no mTLS needed)
- Fixed pre-existing config.rs test (permissions field path, allow→default)
- Removed broken missouri-phase-api test (mTLS incompatible with shell test)
- All 162 clc tests passing, zero warnings

**Remaining:**
- Remove `.clc/state` file creation from `clc workspace init`
- Permission grant storage in DB when coordinator grants (POST /agents/:id/grants)
- Remove all filesystem-based coordination state (permission files, escalation files, cursor files, PID files)
- Worker discovery via DB query instead of .worktrees/ scan

**Files modified:**
- `clc/src/phase.rs` — set_via_api, load_phase_from_api, 5 API tests
- `clc/src/config.rs` — fixed broken test
