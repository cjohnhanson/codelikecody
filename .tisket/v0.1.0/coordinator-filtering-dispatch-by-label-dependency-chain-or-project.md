---
title: "Coordinator filtering: dispatch by label, dependency chain, or project"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:42Z
updated: "2026-03-03T03:02:26Z"
---

## Problem

Coordinators currently grab all todo tiskets or a single named one (`--tisket`). No way to say "work on everything labeled `infrastructure`" or "work on this epic's dependency chain" or "work on project v0.2.0".

## Design

Filtering options for `clc coordinate`:

- `--label <label>` — only tiskets with this label
- `--depends-on <id>` — only tiskets in the dependency chain rooted at this id (enables epic-scoped coordinators)
- `--project <project>` — only tiskets in this project
- `--exclude-label <label>` — skip tiskets with this label (e.g., `--exclude-label needs-human`)
- Filters compose: `--label infrastructure --project v0.1.0`

The coordinator's system prompt should include its filter so it knows its scope and doesn't try to grab everything.

## Implementation

- `find_pickable_tiskets()` in coordinate.rs already does basic filtering
- Extend with label/project/dependency filters using tisket's existing metadata
- Dependency chain walk: given a root tisket, find all transitive dependents that are todo
