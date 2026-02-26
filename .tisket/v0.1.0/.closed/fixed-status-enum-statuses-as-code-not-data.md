---
title: "Fixed status enum — statuses as code not data"
status: done
priority:
assignee:
labels: [architecture, tisket]
depends_on: []
created: 2026-02-26T03:15:50Z
updated: "2026-02-26T04:13:52Z"
---

Statuses drive workflow behavior (pickup gating, phase transitions, abandon
targets) so they should be a fixed enum, not arbitrary per-project strings.

## Current

- Statuses are free-form strings configured per-project in `project.yml`
- `active` / `terminal` classification is per-project config
- clc hardcodes `"todo"` in pickup check — fragile coupling
- `discovery` is used everywhere but isn't even in the valid status list

## Proposed

Fixed enum:

- `discovery` — pre-work, not ready to pick up
- `todo` — ready for work
- `in_progress` — actively being worked on
- `blocked` — was started, needs something before continuing
- `done` — finished
- `cancelled` — intentionally dropped

## Implications

- Remove `statuses` config from `project.yml`
- `active` / `terminal` classification becomes implicit in the enum
- clc matches on variants directly — no mapping table needed
- Arbitrary categorization moves to labels/tags (tags tisket TBD)
- Migration path for existing issues with non-standard statuses
