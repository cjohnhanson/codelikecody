---
title: "supervisor rehydrates agents without validating local branch exists"
status: in_progress
priority:
assignee:
labels: [clc, supervisor, bug]
depends_on: []
created: 2026-04-09T03:21:00Z
updated: "2026-04-09T03:22:26Z"
---

## Problem

At supervisor startup, agents in the coordination DB from previous runs get rehydrated into new Docker containers without checking whether their local worktree branch still exists. When reviewers run against those branches, they fail because the branch doesn't exist locally.

## Evidence

From clc up run on 2026-04-08:

```
supervisor: found 28 agent(s) in DB
supervisor: reset 14 stale agent(s) from prior run
supervisor: worker '1fkl-...' started in Docker
... 14 workers started, none with fresh dispatch events ...
supervisor: reviewer 'test-review' for '8n0e-...': **CHANGES_REQUESTED**
  Cannot perform review: the branch for this worker does not exist locally
supervisor: reviewer 'test-review' for 'h6f3-...': Unable to render a review verdict:
  the branch `h6f3-...` does not exist locally
```

None of the rehydrated tiskets were actually picked up by a coordinator (they remain status 'todo' in tisket). They were resurrected from stale DB state.

## Proposed fix

In supervisor startup, before rehydrating an agent from the DB:
1. Check if the branch `refs/heads/<agent_id>` exists in the project repo via gix
2. If missing, mark the agent as Failed/Stopped and skip container startup
3. Optionally: also check if the worktree directory `.worktrees/<agent_id>` exists, since that's where the work happens

## Acceptance criteria

- Unit test: supervisor helper that decides whether to rehydrate an agent, with cases for (a) branch exists, (b) branch missing, (c) worktree missing
- Integration: a coordination DB with a stale agent whose branch doesn't exist should NOT cause a container to be started on next `clc up`

## Scratch Notes
