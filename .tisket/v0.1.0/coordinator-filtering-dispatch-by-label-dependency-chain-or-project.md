---
title: "Coordinator filtering: dispatch by label, dependency chain, or project"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:42Z
updated: "2026-03-03T03:33:26Z"
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

### Test Design (session 2 — refined)

**Approach:** Missouri state-graph tests. All assertions use `--dry-run` flag (prints pickable IDs to stdout without spawning coordinator). Side-effect-free.

**Fixture (coordinate-filter-setup state):**
- v0.1.0/alpha-task: labels=[infrastructure], status=todo, no deps
- v0.1.0/beta-task: labels=[feature], status=todo, no deps
- v0.1.0/gamma-task: labels=[needs-human], status=todo, no deps
- v0.1.0/dep-root: labels=[infrastructure], status=done, no deps
- v0.1.0/dep-child: labels=[], status=todo, depends_on=[dep-root] (pickable)
- v0.1.0/dep-grandchild: labels=[], status=todo, depends_on=[dep-child] (NOT pickable)
- v0.2.0/delta-task: labels=[feature], status=todo, no deps

**Pickable (no filter):** alpha-task, beta-task, gamma-task, dep-child, delta-task (5)
**Not pickable:** dep-root (done), dep-grandchild (unresolved dep)

**Key design choice:** dep-root is `done` so the dependency chain `dep-root → dep-child → dep-grandchild` tests both transitive walk AND pickability filtering. `--depends-on dep-root` scope = {dep-root, dep-child, dep-grandchild}, pickable = {dep-child}.

**Files written:**
- clc/tests/missouri/coordinate-filter-setup/.missouri/missouri.yml — 44 assertions
- clc/tests/missouri/coordinate-filter-setup/.claude/settings.local.json
- clc/tests/missouri/coordinate-filter-setup/Cargo.toml
- clc/tests/missouri/coordinate-filter-setup/src/main.rs
- clc/tests/missouri/initialized/.missouri/missouri.yml — added transition

**Implementation targets:**
- clc/src/cli.rs — add --label, --exclude-label, --project, --depends-on, --dry-run flags to Coordinate
- clc/src/main.rs — wire new flags through cmd_coordinate
- clc/src/coordinate.rs — extend find_pickable_tiskets with filters, add dry-run path

**Implementation notes:**
- repo.list_issues(project, None, false) already supports project filter for list_issues
- labels in issue.frontmatter.labels: Vec<String>
- depends_on chain: walk transitive dependents scanning all issues' depends_on fields
- Filters compose: label ∩ project ∩ depends-on scope, minus exclude-labels
- --dry-run: print pickable IDs to stdout (one per line) and exit without spawning
- "no pickable tiskets found" goes to stderr (existing behavior)
