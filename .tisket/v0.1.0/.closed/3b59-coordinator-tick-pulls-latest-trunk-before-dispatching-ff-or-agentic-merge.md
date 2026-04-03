---
title: "coordinator tick pulls latest trunk before dispatching — ff or agentic merge"
status: done
priority: 1
assignee:
labels: [clc, clc-up-target, standard]
depends_on: []
created: 2026-04-03T18:39:14Z
updated: "2026-04-03T18:44:25Z"
---

## Problem

Coordinators in Docker receive a git pack at startup and never refresh.
Tisket label changes, new tiskets, and landed work on trunk are invisible
until the coordinator container is restarted. This defeats the purpose of
a long-running supervisor — every metadata change requires killing and
restarting `clc up`.

The supervisor does `git pull` equivalent operations when landing work
(importing packs, ff-merging to trunk). The same mechanism needs to run
in the other direction: trunk changes need to flow into the coordinator's
workspace.

## Proposed solution

At the top of each coordinator tick (before `find_undispatched`):

1. Fetch latest trunk from the supervisor API (`GET /git/pack/main`)
2. Attempt fast-forward merge into the coordinator's local main
3. If ff succeeds, continue with updated tisket metadata
4. If ff fails (conflict), invoke Claude to resolve the merge — same
   pattern as landing conflicts but in reverse

For local (non-Docker) coordinators, this is a `git pull --ff-only`
on the host repo, falling back to agentic merge on conflict.

The supervisor API already has `GET /git/pack/{branch}` which creates
a pack from the host repo. The coordinator already has the git pack
import machinery. The pieces exist — they just need to be wired into
the tick loop.

## Done When

- Coordinator tick starts with a trunk refresh before reading tiskets
- New tiskets added to trunk appear in the coordinator's pickable list
  without restarting `clc up`
- Label changes on tiskets take effect within one poll cycle
- Landed work (new commits on trunk) is visible to the coordinator
- Fast-forward refresh doesn't block the tick loop for more than a few
  seconds (pack transfer + ff merge)
- Conflicts fall back to agentic merge (Claude session in the coordinator)
- At least one test verifies that a tisket added after coordinator startup
  is picked up on the next tick
