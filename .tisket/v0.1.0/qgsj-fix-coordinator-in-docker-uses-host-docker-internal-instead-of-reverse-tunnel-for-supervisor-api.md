---
title: "fix: coordinator in Docker uses host.docker.internal instead of reverse tunnel for supervisor API"
status: in_progress
priority:
assignee:
labels: [clc, clc-up-target]
depends_on: []
created: 2026-03-26T23:22:00Z
updated: "2026-03-26T23:22:12Z"
---

## Scratch Notes

### Session 2026-03-27 — clc up e2e

**What works (verified across 3+ restarts):**
- `clc up` → supervisor → coordinator in Docker via SSH → mTLS via host.docker.internal
- Coordinator finds pickable tiskets, dispatches via `POST /dispatch` on supervisor API
- Supervisor picks up Pending workers, starts Docker workspaces via SSHWorkspace
- Stale agent cleanup on supervisor restart
- Re-dispatch of stopped agents (dispatch endpoint re-activates)
- Worker receives git pack, creates tisket branch, starts Claude
- Claude authenticates via OAuth token from `~/.claude/token`
- Git index written via `gix::index::State::from_tree` after pack unpack
- Symlinks handled in `checkout_tree`
- Supervisor landing flow: SSH into worker → export pack → import → ff_merge to trunk
- Coordinator skips landing for Docker workspaces (supervisor owns trunk)

**RESOLVED: replaced custom pack format with tar of .git directory**
The custom pack writer/parser was the root cause — it didn't include symlink
blob objects, causing checkout_tree to abort mid-walk. The tar approach
transfers the real .git directory (pack files, indices, refs) that gix reads
natively. Verified: dirty: 0, all files present including symlinks.

**Previous blocker (resolved): checkout_tree regression**
After merging main (which has new missouri commits) into the worktree branch,
the git pack `checkout_tree` only writes a few top-level directories (`.agents/`,
`.claude/`, `.clc/`) instead of the full tree. `tisket.yml`, `Cargo.toml`, all
source dirs — missing. The pack is ~10MB so the objects are there; the tree walk
is incomplete.

This WAS working in earlier runs (before the main merge). Likely the new commits
changed the tree structure in a way that exposes a bug in `checkout_tree` or
`unpack_to_loose`. Could be OFS_DELTA/REF_DELTA objects not being resolved
correctly for the new tree entries.

**Files changed on qgsj branch (16 commits):**
- `clc/src/ssh_workspace.rs` — host.docker.internal for coordinators+workers, SSHWorkspace::exec(), always transfer main branch, tisket_id for --branch arg, 2>&1 on worker exec
- `clc/src/ssh_session.rs` — removed dead keepalive code
- `clc/src/supervisor.rs` — WorkerState, _workspace retention, stale agent cleanup, start_pending_workers, start_worker_docker, land_completed_workers, OAuth token from ~/.claude/token
- `clc/src/supervisor_api.rs` — dispatch re-activates stopped agents, removed unused import
- `clc/src/coordinator_loop.rs` — stderr flush, dispatched filter by Running|Pending, Docker coordinators skip landing, removed dead import_workspace_pack
- `clc/src/coordination_client.rs` — multi_thread runtime for API client
- `clc/src/git_pack.rs` — write_index_from_tree via State::from_tree, symlink handling in checkout_tree
- `clc/src/gix_ops.rs` — create_branch writes ref file directly (no reflog)
- `clc/src/main.rs` — workspace start creates+checkouts tisket branch, sets phase
- `moose/src/native/mod.rs` — removed dangling test module declarations

**Session 2 (2026-03-27 ~08:00):**
- Merged 86or (workflow engine, bearer tokens, review system)
- Built proper permission flow: phase guard → API grant check → escalation
- Coordinator seeds baseline grants (BASELINE_TOOL_GRANTS) at dispatch via API
- Phase init routes through API (no .clc/state file), retries on startup race
- Worker exec sets mTLS cert env vars using step-4 deployed certs
- Stop hook allows stopping when permission escalation pending
- Verified: 24 grants seeded, phase set via API, Claude starts and uses tools
- Claude stalled after ~200 tool calls — investigating clc infra instead of
  doing assigned task. Likely hit a phase guard block, misinterpreted it,
  went down a rabbit hole. Behavioral, not infrastructure.
- Landing flow still untested (needs completed worker)

**Session 3 (2026-03-27):**
- Fixed phase transitions in Docker workers: `set_with_workflow` now routes
  through API when `CLC_API_URL` is set. Before: `.clc/state` didn't exist
  in container, so transitions failed silently.
- Supervisor sets worker status to Running after Docker start. Before:
  workers stayed Pending, coordinator saw running=0 and could over-dispatch.
- Docker coordinator health check uses DB status, not PID.

**First live run with phase fix:**
- Phase transitions work! tests-unwritten → tests-written → ... → done
- Worker wrote tests, committed, advanced through all 9 phases
- Worker called `clc done` successfully, status set to Completed
- **Supervisor landing blocked:** `clc workspace export` outputs ~20MB
  JSON to stdout which blocks when piped through SSH exec (channel buffer fills)
- Fixed by adding `--output` flag: write to file on container, then
  read back via `cat` over SSH

**Test tisket:** `8z9n` — add unit tests for tisket Status methods (labeled clc-up-target)

**~34 commits on qgsj branch** — original URL fix grew into full clc up pipeline.
