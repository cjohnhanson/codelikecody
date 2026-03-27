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

**Final verified state (2026-03-27 ~23:00):**
- Full e2e: supervisor → coordinator → dispatch → worker → Claude running
- Worker: correct branch, dirty: 0, tools working (Read, Bash confirmed)
- Claude hit rate limit and looped — operational issue, not a code bug
- Landing flow written but untested (needs completed worker to trigger)

**Next steps:**
1. Re-run with fresh API quota — Claude should complete the tisket
2. Verify landing: supervisor fetches tar from worker, imports, ff-merges to trunk
3. Test restart resilience with the tar approach

**Test tisket:** `8z9n` — add unit tests for tisket Status methods (labeled clc-up-target)

**20 commits on qgsj branch** — original URL fix grew into full clc up pipeline:
coordinator API connectivity, workspace retention, stale agent cleanup,
worker launcher, landing flow, OAuth token passing, branch creation,
and replacing the custom pack format with tar of .git directory.
