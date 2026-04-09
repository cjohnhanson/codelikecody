---
title: "stale 'test-worker' escalation messages spam supervisor log (8424 lines per run)"
status: in_progress
priority:
assignee:
labels: [clc, supervisor, bug]
depends_on: []
created: 2026-04-09T03:21:02Z
updated: "2026-04-09T03:52:07Z"
---

## Problem

Running `clc up` produces thousands of log lines like:

```
[ESCALATION] worker 'test-worker': needs docker
  Grant: clc permissions grant test-worker "<permission>"
  Deny:  clc permissions deny test-worker "<reason>"
[ESCALATION] worker 'test-worker': first request
[ESCALATION] worker 'test-worker': second request
[ESCALATION] worker 'test-worker': needs docker access
```

From a single clc up session that ran ~2 minutes: **8424 test-worker escalation messages out of 13918 total log lines (60% of log volume).**

There is no worker named 'test-worker' being dispatched. These are stale messages in the coordination DB queue.

## Suspected causes

1. Test harness that used agent_id 'test-worker' leaked messages into the production coordination DB (likely `.clc/coordination.db`)
2. Those messages never get marked as consumed (no cursor advancement for permission escalation messages?)
3. Every supervisor tick re-reads the pending escalations and re-prints them

## Proposed fix

Two parts:
1. Figure out the bug that causes these messages to persist. Check how permission escalation cursor/consumption works.
2. Add a one-time cleanup: at supervisor startup, delete messages for agents that aren't in the current agents table (or just for known test IDs like 'test-worker').

## Acceptance criteria

- Log volume from a clean clc up run has no test-worker entries
- If a message is shown to a human (escalation printed), it gets marked as shown so it doesn't repeat

## Scratch Notes
