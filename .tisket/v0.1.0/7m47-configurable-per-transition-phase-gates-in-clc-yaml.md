---
title: "configurable per-transition phase gates in clc.yaml"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-23T00:27:17Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

Phase transitions should be configurable per-project — different projects have different workflow needs, and a one-size-fits-all phase sequence forces either too much ceremony or too little rigor depending on context.

The phase system in `clc/src/phase.rs` has a hardcoded `Phase::ALL` array defining exactly nine phases in a fixed order (`TestsUnwritten` through `Done`). The `phase::set()` function validates transitions against this static ordering — forward by exactly one step, backward to any earlier step. The config system (`clc/src/config.rs`) has `workflows` and `rules` fields that define named workflow phase sequences and match criteria, but the actual `phase::set()` function does not consult the config at all. The workflow resolution (`config.resolve_workflow()`) returns a `WorkflowDef` with a `phases: Vec<String>`, but this is never wired into transition validation.

Projects that don't need the full nine-phase TDD+review cycle (e.g., documentation, triage, infrastructure) are forced through phases that don't apply, while projects that need custom gates (e.g., security review before merge) can't add them.

## Open Questions

- Should configurable workflows replace the hardcoded `Phase` enum entirely, or should the enum remain as a superset with workflows selecting a subset?
- How do per-transition gates interact with the existing `required_attempts` mechanism?
- Where in the call chain should the workflow config be injected — should `phase::set()` accept a workflow parameter, or should a higher-level function handle validation?

## Why It Matters

The workflow config infrastructure exists in `config.rs` but is inert — it's parsed, serialized, and stored, but never used to gate actual transitions. The gap between what's configurable and what's enforced creates false expectations for users who configure workflows expecting them to take effect.
