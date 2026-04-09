---
title: "max_workers not enforced — 14 workers ran with config cap of 5"
status: in_progress
priority:
assignee:
labels: [clc, supervisor, bug]
depends_on: []
created: 2026-04-09T03:21:03Z
updated: "2026-04-09T04:00:06Z"
---

## Problem

The `clc.yaml` config specifies per-coordinator worker caps:

```yaml
coordinators:
  auto:
    max_workers: 3
  standard:
    max_workers: 2
```

Expected total: 5 workers. Observed: 14 workers running in Docker containers during a clc up session.

## Important caveat

The 14 workers observed were **rehydrated from stale coordination DB state** (see tisket ghzk), not freshly dispatched via coordinator selectors. It's possible the rehydration path simply doesn't go through max_workers enforcement.

So this bug may be:
(a) max_workers genuinely not enforced anywhere, or
(b) max_workers enforced only on fresh dispatch, with a separate rehydration path that ignores it, or
(c) Correct behavior that I misread

## Needs verification

Reproduce on a clean coordination DB (see ghzk for why the DB was dirty). If 14 workers still exceed 5, the cap is broken. If only ≤5 run, then the issue is specifically in rehydration and is covered by ghzk.

## Acceptance criteria

- Write a test that verifies a coordinator with max_workers=1 never dispatches a second worker while the first is running
- Fix any path that bypasses the cap

## Scratch Notes
