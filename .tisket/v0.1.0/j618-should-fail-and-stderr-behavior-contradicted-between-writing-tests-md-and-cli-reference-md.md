---
title: "should_fail and stderr behavior contradicted between writing-tests.md and cli-reference.md"
status: todo
priority:
assignee:
labels: [missouri, docs, accuracy, auto]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-04-03T18:32:46Z"
---

## Problem

When `should_fail: true` is set on an assertion, the docs should agree on whether `stdout` and `stderr` matching still applies. writing-tests.md shows `should_fail` combined with `stderr` matching (line 372: `should_fail: true` alongside `stderr: "error: already initialized..."`), implying both work together. cli-reference.md states the opposite (line 294: "Stdout/stderr matching is skipped in this mode"). The executor (executor.rs line 1811) confirms cli-reference.md is correct — when `should_fail` triggers, the function returns early with `passed: true` and no stdout/stderr comparison.

An agent writing tests from writing-tests.md will add `stderr` assertions alongside `should_fail: true`, expect them to be checked, and never notice they're silently ignored.

## Open Questions

- Should the executor be changed to support stderr matching with should_fail (the writing-tests.md behavior), or should writing-tests.md be corrected to match the actual behavior?
- Are there existing test suites in the repo relying on the combined pattern that silently pass without checking stderr?

## Why It Matters

Tests that appear to verify error messages are actually verifying nothing. Silent false-passes erode confidence in the test suite.

## Scratch Notes

### Decision
The fix should make the executor check stderr/stdout even when should_fail is true (match writing-tests.md behavior). cli-reference.md and the executor need to be updated.

### Key files
- `missouri/src/executor.rs:1866-1887` — should_fail early return skips stdout/stderr comparison
- `missouri/src/graph.rs:78-91` — Assertion struct
- `missouri/docs/writing-tests.md:367-372` — shows should_fail + stderr (desired behavior)
- `missouri/docs/cli-reference.md:295-328` — documents should_fail (needs update)

### Test plan
- Unit tests in executor.rs testing run_single_assertion with should_fail + stderr/stdout
- Missouri integration test with should_fail + stderr matching

### Status
- Phase: review-requested
- Tests committed (4 unit tests in executor.rs)
- Implementation: changed should_fail early return to fall-through for stdout/stderr comparison
- Docs: updated cli-reference.md should_fail description
- Existing tests using should_fail+stderr pattern: many across belmont, tisket, zettel (all should still pass since they have correct stderr expectations)
- Could not run cargo test (no permission) — unit tests verified by code analysis
