---
title: "hook.rs assemble_prime and assemble_reinforcement have zero tests — primary behavioral contract untested"
status: todo
priority:
assignee:
labels: [clc, testing, blocking, standard]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-04-03T18:33:27Z"
---

## Problem

1. `assemble_prime` and `assemble_reinforcement` in `clc/src/hook.rs` (404 lines) define what every agent sees at session start and on every prompt — the prime text that sets behavioral constraints, phase state, tisket context, and trunk directives. These functions should have unit tests verifying their output varies correctly by git state, phase, config, and branch type.
2. Neither function has any test coverage. There are zero `#[test]` attributes and no `#[cfg(test)]` module anywhere in `hook.rs`. The functions are `fn` (not `pub`), making them untestable from outside the module without refactoring.
3. Changes to prime or reinforcement assembly — the primary behavioral contract between clc and its agents — can ship with no automated verification that the output is correct. Regressions in context injection silently alter agent behavior in ways that only surface as mysterious workflow failures.

## Open Questions

- Should tests live as unit tests inside `hook.rs` (requiring the functions to remain private) or as integration tests (requiring `pub` visibility or a test helper)?
- What's the minimum set of scenarios: trunk vs feature branch, phased vs unphased, tisket present vs absent, config with skills vs without?
- Can `assemble_prime` be tested without a real git repo, or does it need fixture state?

## Why It Matters

Prime text is the single strongest lever over agent behavior. Untested assembly means any refactor — config format changes, new phase gates, skill injection tweaks — can silently break the agent contract with no signal until a human notices workers misbehaving.
