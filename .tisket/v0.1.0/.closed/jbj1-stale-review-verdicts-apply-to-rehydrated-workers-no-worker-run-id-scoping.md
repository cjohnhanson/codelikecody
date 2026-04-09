---
title: "stale review verdicts apply to rehydrated workers — no worker_run_id scoping"
status: done
priority:
assignee:
labels: [clc, supervisor, bug]
depends_on: []
created: 2026-04-09T03:21:02Z
updated: "2026-04-09T03:51:50Z"
---

## Problem

When a worker is rehydrated from a previous run, stale review verdict messages in the coordination DB from that previous run are still applied to the rehydrated worker. The supervisor sees 'all reviews approved' and advances the phase — even though the current worker has done no work.

## Evidence

From clc up run on 2026-04-08:

```
supervisor: all reviews approved for 'kq0i-workspace-encapsulation-...',
  advancing tests-unwritten → tests-written
supervisor: resuming 'kq0i-...' after review
```

kq0i is still status 'todo' in tisket (never picked up by a coordinator). The approval came from stale DB messages.

## Proposed fix

Review verdicts should be scoped to a specific worker run, not to the agent_id alone. Options:

1. When rehydrating an agent, clear all prior review messages for that agent_id.
2. Add a `worker_run_id` (UUID generated at spawn time) to review request and review verdict messages; only count verdicts whose run_id matches the current run.
3. Use message timestamps: require verdicts to have a timestamp newer than the current worker's start time.

Option 2 is the cleanest — it also handles in-flight scenarios where a worker restart shouldn't lose its in-flight reviews.

## Acceptance criteria

- Unit test: rehydrating an agent with stale ReviewResult messages in DB → pending_reviews returns empty (or matches only current-run verdicts)
- Regression: kq0i-style advancement from stale approvals should not happen
