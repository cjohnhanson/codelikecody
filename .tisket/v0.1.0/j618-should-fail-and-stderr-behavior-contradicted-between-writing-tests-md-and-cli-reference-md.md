---
title: "should_fail and stderr behavior contradicted between writing-tests.md and cli-reference.md"
status: todo
priority:
assignee:
labels: [missouri, docs, accuracy]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

When `should_fail: true` is set on an assertion, the docs should agree on whether `stdout` and `stderr` matching still applies. writing-tests.md shows `should_fail` combined with `stderr` matching (line 372: `should_fail: true` alongside `stderr: "error: already initialized..."`), implying both work together. cli-reference.md states the opposite (line 294: "Stdout/stderr matching is skipped in this mode"). The executor (executor.rs line 1811) confirms cli-reference.md is correct — when `should_fail` triggers, the function returns early with `passed: true` and no stdout/stderr comparison.

An agent writing tests from writing-tests.md will add `stderr` assertions alongside `should_fail: true`, expect them to be checked, and never notice they're silently ignored.

## Open Questions

- Should the executor be changed to support stderr matching with should_fail (the writing-tests.md behavior), or should writing-tests.md be corrected to match the actual behavior?
- Are there existing test suites in the repo relying on the combined pattern that silently pass without checking stderr?

## Why It Matters

Tests that appear to verify error messages are actually verifying nothing. Silent false-passes erode confidence in the test suite.
