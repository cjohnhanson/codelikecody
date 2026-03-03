---
title: "Multiple concurrent coordinators with tisket claim registry"
status: done
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:44Z
updated: "2026-03-03T04:05:31Z"
---

## Problem

With coordinator filtering, multiple coordinators can run simultaneously (one for `infrastructure`, one for `ui`, etc.). They need to not grab the same tiskets. Currently there's no claim mechanism — two coordinators checking `tisket issue list` at the same time could dispatch the same tisket.

## Design

### Claim registry

When a coordinator dispatches a worker for a tisket, it claims it:
- Tisket status advances to `in_progress` (already happens)
- Tisket gets an `assignee` or `coordinator` field set to the coordinator's id

Before dispatching, coordinators check if a tisket is already claimed by another coordinator. The tisket's `in_progress` status + assignee field is the claim.

### Coordinator identity

Each coordinator gets a unique id at launch (e.g., `coord-infra`, `coord-ui`, or auto-generated). This id is used for:
- Claim attribution
- Worker directory namespacing
- Branch naming (`integrate/coord-infra/<timestamp>`)
- Admin session management (`clc coordinator coord-infra check`)

### Conflict resolution

If two coordinators race on the same tisket:
- First to set `in_progress` + assignee wins
- Second sees it's claimed and skips
- File-level atomicity in `.tisket/` is sufficient (git worktrees have independent working trees)

## Depends on
- `coordinator-filtering-dispatch-by-label-dependency-chain-or-project`
- `git-workflow-ephemeral-integration-branch-with-squash-merge-landing`

## Scratch Notes

### Phase: done

#### Test design decisions

- Using existing `assignee` field in IssueFrontmatter (already exists, `Option<String>`)
- New CLI flags: `--coordinator-id` on `clc dispatch`, `--id` on `clc coordinate`
- On dispatch with `--coordinator-id`, the tisket assignee is set to the coordinator's ID
- assignee field persists after worker stop (attribution survives lifecycle)

#### Missouri test state graph

```
initialized → claim-ready → claimed-via-dispatch → claimed-worker-stopped
```

- `claim-ready`: two tiskets — feat-alpha (todo, unclaimed), feat-beta (in_progress, assignee: coord-ui)
  - Assertions: tisket states, CLI --help shows new flags
- `claimed-via-dispatch`: after `clc dispatch feat-alpha --coordinator-id coord-infra`
  - Assertions: assignee set to coord-infra, worktree+worker exist, feat-beta unchanged
  - Also asserts: dispatch of already-claimed tisket by different coordinator fails
- `claimed-worker-stopped`: after stopping the worker
  - Assertions: assignee preserved, status preserved, worktree intact

#### Key files consulted

- `clc/src/cli.rs` — Coordinate/Dispatch CLI definitions
- `clc/src/dispatch.rs` — dispatch flow (pickup → permissions → spawn)
- `clc/src/pickup.rs` — pickup flow (status check → edit_issue → worktree)
- `clc/src/coordinate.rs` — coordinator loop, find_pickable_tiskets
- `clc/src/worker.rs` — COORDINATOR_ID constant, worker_dir_for
- `tisket/src/issue.rs` — IssueFrontmatter struct (assignee already exists)
- `tisket/src/repo.rs` — edit_issue (currently only sets status, not assignee)

#### Implementation plan (next phase)

1. Add `--coordinator-id` flag to `Dispatch` in cli.rs
2. Add `--id` flag to `Coordinate` in cli.rs
3. Extend `dispatch()` to accept coordinator_id, pass to pickup
4. Extend `pickup()` to accept coordinator_id, set assignee via repo.edit_issue
5. Extend `repo.edit_issue()` to accept optional assignee parameter
6. Wire coordinate → dispatch with coordinator ID
