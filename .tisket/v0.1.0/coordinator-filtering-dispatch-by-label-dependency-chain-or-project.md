---
title: "Coordinator filtering: dispatch by label, dependency chain, or project"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:42Z
updated: "2026-03-03T03:32:43Z"
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

## Scratch Notes

### Test Design (session 1)

**Approach:** Missouri state-graph tests. Side-effect-free assertions only (no coordinator spawning).

**Fixture (coordinate-filter-setup state):**
- v0.1.0/alpha-task: labels=[infrastructure], todo, no deps
- v0.1.0/beta-task: labels=[feature], todo, no deps
- v0.1.0/gamma-task: labels=[needs-human], todo, no deps
- v0.1.0/dep-parent: labels=[infrastructure], todo, no deps
- v0.1.0/dep-child: labels=[], todo, depends_on=[dep-parent]
- v0.2.0/delta-task: labels=[feature], todo, no deps

**Key test strategy:** Compose new filters with existing --tisket flag to verify positive filtering without spawning coordinators. Test negative cases via "no pickable tiskets" stderr message.

**Files:**
- clc/tests/missouri/coordinate-filter-setup/ — new state with 16 assertions
- clc/tests/missouri/initialized/.missouri/missouri.yml — transition to new state
- clc/src/coordinate.rs — implementation target (find_pickable_tiskets)
- clc/src/cli.rs — new CLI flags
- clc/src/main.rs — wire new flags through cmd_coordinate

**Implementation notes:**
- repo.list_issues(project, None, false) already supports project filter
- labels in issue.frontmatter.labels: Vec<String>
- depends_on chain: walk transitive dependents scanning all issues
- Filters compose: label ∩ project ∩ depends-on scope, minus exclude-labels
