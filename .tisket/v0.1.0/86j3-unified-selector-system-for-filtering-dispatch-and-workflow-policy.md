---
title: "Unified selector system for filtering, dispatch, and workflow policy"
status: in_progress
priority:
assignee:
labels: [tisket, clc, feature]
depends_on: []
created: 2026-03-09T02:17:58Z
updated: "2026-03-19T02:40:15Z"
---

## Context

Tisket has labels, projects, status, and (soon) arbitrary tags — all
queryable attributes on issues. Currently each has its own filter flag
(`--label`, `-p`, `--status`) with no composition. Coordinators filter
by `--label` only. clc enforces one hardcoded TDD workflow for all work.

Three problems that share a root cause:
- No way to mark work as non-code (docs, admin) and skip TDD phases
- Coordinator dispatch can only filter by label, not by project/status/tags
- Search, list, and dispatch each have their own filter logic

## Design

### Layer 1: Selector grammar (tisket)

Uniform `namespace:value` syntax across all query surfaces:

    label:docs  project:v0.1.0  status:todo  estimate:4

Multiple selectors AND together. Same syntax works for:
- `tisket issue list`
- `tisket search`
- `clc coordinate --filter`
- Workflow policy rules in config

Built-in namespaces with special semantics: `label`, `project`, `status`.
Everything else queries the `tags` map from the configurable-fields tisket.

### Layer 2: Workflow policies (clc)

In `.clc/config.yml`, selectors map to workflow definitions:

    workflows:
      default:
        phases: [tests-unwritten, tests-written, red, implementing, green]
      lightweight:
        phases: []

    rules:
      - match: { label: docs }
        workflow: lightweight
      - match: { label: admin }
        workflow: lightweight

First matching rule wins. Unmatched falls through to `default`.

When `clc pickup` grabs a tisket, it evaluates the tisket attributes
against the rules and sets the appropriate phase sequence.

### Layer 3: Coordinator scoping

Generalizes existing `--label` to the full selector grammar. A coordinator
can scope to `label:feature,project:v0.1.0` and only dispatch matching
tiskets.

## Dependencies

- configurable-fields-per-project (tags storage + --where filtering)

## Scope

- Selector grammar: parsing, evaluation against issue attributes
- Integration into tisket list, search
- Integration into clc coordinate dispatch
- Workflow policy config parsing and enforcement in clc pickup/phases
- At least two built-in workflows: default (TDD) and lightweight (no phases)

## Out of scope

- OR composition (use multiple coordinators instead)
- Custom phase definitions beyond ordering (custom gate logic)
- Tag schema validation

## Scratch Notes
