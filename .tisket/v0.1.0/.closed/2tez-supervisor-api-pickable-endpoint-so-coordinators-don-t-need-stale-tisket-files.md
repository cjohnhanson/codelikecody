---
title: "supervisor API: /pickable endpoint so coordinators don't need stale tisket files"
status: done
priority: 2
assignee:
labels: [clc, auto]
depends_on: []
created: 2026-04-03T19:28:09Z
updated: "2026-04-03T20:02:32Z"
---

## Problem

Coordinators in Docker read tisket files from a git pack received at
startup. Even with trunk refresh, the coordinator transfers ~18MB of pack
data on every tick just to check which tiskets are pickable. The supervisor
already has the latest trunk — the coordinator should ask it instead of
maintaining its own copy of the tisket metadata.

## Proposed solution

Add `GET /pickable` to the supervisor API. Query params:

- `label` — only tiskets with this label (optional)
- `exclude_label` — exclude tiskets with this label (optional)
- `project` — tisket project filter (optional)

The supervisor reads tiskets from its host repo (always current), applies
the filters, checks `is_pickable()` and `depends_on` resolution, and
returns a JSON array of tisket IDs.

The coordinator's `find_undispatched` calls this endpoint when
`CLC_API_URL` is set instead of opening the tisket repo locally.

## Done When

- `GET /pickable?label=auto&project=v0.1.0` returns pickable tisket IDs
- Coordinator in Docker uses the API endpoint instead of local tisket files
- Local coordinators still read tiskets directly (no API needed)
- Filtering matches the existing `find_undispatched` logic (label, exclude_label, project, depends_on, is_pickable)
- Response excludes tiskets already dispatched by this coordinator
- At least one supervisor API test verifies the endpoint
