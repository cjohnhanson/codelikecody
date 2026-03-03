---
title: "Multiple concurrent coordinators with tisket claim registry"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:44Z
updated: "2026-03-03T03:32:40Z"
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
