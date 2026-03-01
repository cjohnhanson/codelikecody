---
title: "Review phase: coordinator-gated code review before merge"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-01T13:04:59Z
updated: "2026-03-01T16:52:18Z"
---

Add a review phase to the worker lifecycle so the coordinator gates what lands.

## Flow

1. Worker reaches green, advances to `review-requested`, stops
2. Coordinator sees `review-requested`, dispatches review worker, phase becomes `in-review`
3. Review worker reads diff, checks quality, writes findings to `.clc/review/`, advances to `reviewed`, stops
4. Coordinator sees `reviewed`, reads `.clc/review/`, decides:
   - Accept: advances to `done`, calls `clc done`, merges
   - Reject: kicks back to `implementing`, writes rejection notes, dispatches fix worker

## Key constraints

- Review worker produces a report, never a verdict. Coordinator is the only merge authority.
- `.clc/review/` artifacts must accumulate across review cycles (timestamped or indexed), not overwrite.
- Review worker prompt must include Missouri context — checking that test coverage is exhaustive, real, and meaningful is a minimum baseline.
- The review worker needs full context on what Missouri is and how the project's test suites work.

## New phases

- `review-requested` (between green and done, Stop hook allows exit)
- `in-review` (coordinator sets when dispatching reviewer)
- `reviewed` (reviewer sets after writing findings)

## Changes needed

- phase.rs: add three new phases
- guard.rs: Stop hook allows exit at `review-requested` and `reviewed`
- coordinate.rs: detect `review-requested`, dispatch reviewer, read `reviewed` artifacts, make accept/reject decision
- done.rs: only callable from `done` phase (coordinator advances to done before calling)
- New: review worker prime text / prompt
