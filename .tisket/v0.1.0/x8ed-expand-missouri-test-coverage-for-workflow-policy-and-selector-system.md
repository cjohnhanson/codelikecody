---
title: "expand missouri test coverage for workflow policy and selector system"
status: in_progress
priority: 2
assignee:
labels: [clc, missouri]
depends_on: []
created: 2026-03-22T01:07:43Z
updated: "2026-03-22T01:08:09Z"
---

## Existing coverage

- `has-workflow-policy-config`: config parses, workflows/rules sections in output
- `picked-up-lightweight`: label:docs issue → lightweight workflow → phase:done, clc done succeeds

## Gaps to fill

- Default workflow: feature-labeled issue picked up → standard TDD phase sequence
- Rule precedence: multiple rules, first match wins
- Status match criteria in rules
- Pickup without any workflow config → falls back to hardcoded TDD phases
- Pickup with config but no matching rule → falls through to default workflow
- Coordinator selector filtering: dispatch only matching issues

## Scratch Notes
