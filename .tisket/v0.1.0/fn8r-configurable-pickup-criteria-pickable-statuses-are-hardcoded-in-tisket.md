---
title: "configurable pickup criteria — pickable statuses are hardcoded in tisket"
status: todo
priority:
assignee:
labels: [enhancement, tisket]
depends_on: []
created: 2026-04-04T12:28:44Z
updated: "2026-04-04T12:53:07Z"
---

## Problem

`Status::is_pickable()` in `tisket/src/issue.rs:36` hardcodes
`Todo | Blocked | Paused` as the only pickable statuses. Coordinators
filter by `is_pickable()` and then by label/project selectors, but
there's no way to configure which statuses count as pickable.

A coordinator that only picks up `blocked` issues for triage, or a
workflow that treats `discovery` issues as pickable for investigation
agents, can't be expressed in the current system. The hardcoded set
assumes one workflow shape.

## Acceptance Criteria

- [ ] Given a `tisket.yml` with a `pickable_statuses` field (or
      equivalent), when `is_pickable()` is called, then only the
      configured statuses return true
- [ ] Given no explicit config, when `is_pickable()` is called,
      then the current default (todo, blocked, paused) is preserved
- [ ] Given a coordinator with a `selector.status` field in
      `clc.yml`, when the coordinator polls for work, then only
      issues matching that status are considered

## Out of Scope

- Changing the status lifecycle itself (adding/removing statuses)
- Per-issue override of pickability

## Done When

- Pickable statuses are configurable via `tisket.yml` or `clc.yml`
  or both
- Default behavior is unchanged when no config is present
- At least one test covers custom pickable status configuration
- Existing tests pass
