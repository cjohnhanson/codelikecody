---
title: "clc-api missouri test states use generic state-a/state-b names"
status: discovery
priority: 4
assignee:
labels: []
depends_on: []
created: 2026-03-21T18:35:59Z
updated: "2026-03-23T02:14:05Z"
---

## Problem

Missouri test state directories should have descriptive names that communicate what the state represents — the name is the primary documentation for what a test fixture looks like at each point in the workflow.

The clc-api missouri test fixtures (`clc-api/tests/missouri/`) all use generic `state-a`/`state-b` directory names for their transition states. For example, `api-health/state-a` transitions to `api-health/state-b`, `api-create-issue/state-a` transitions to `api-create-issue/state-b`, and so on across all five test scenarios (health, list-issues, create-issue, edit-issue, close-reopen).

When reading or debugging these tests, `state-a` and `state-b` convey nothing about what changed between states. Descriptive names like `server-running` → `health-checked`, or `has-issue` → `issue-edited` would make the fixtures self-documenting and consistent with the naming convention used in other missouri test suites (e.g., tisket tests use `has-issue` → `issue-closed`, zettel tests use `initialized` → `has-note`).

## Open Questions

- Should the rename follow the pattern `<precondition>` → `<postcondition>` consistently, or is there a better convention for API test states?
- Do any transition scripts or missouri.yml files reference the directory names in a way that would need updating beyond the directory rename?

## Why It Matters

Generic state names are a readability tax on every person (or agent) who reads or modifies these tests. The cost is small per instance but compounds across the entire fixture set, and sets a bad precedent for new tests.
