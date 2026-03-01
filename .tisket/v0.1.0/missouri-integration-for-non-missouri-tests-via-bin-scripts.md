---
title: "Missouri integration for non-missouri tests via bin scripts"
status: discovery
priority: 2
assignee:
labels: [missouri]
depends_on: []
created: 2026-02-26T04:24:25Z
updated: "2026-03-01T16:52:16Z"
---

Merge guards and workflow checks should require ALL tests pass, not just missouri
state-graph tests. Projects have cargo tests, clippy, fmt — things that aren't
state graphs but still gate correctness.

Missouri should be the single entry point for "do tests pass." Non-missouri tests
get integrated via `.missouri/bin/` scripts that missouri invokes. For example, a
`cargo-test` bin script that runs `cargo test --workspace` and reports pass/fail
in a way missouri understands.

This makes `missouri run` the one command that answers "is everything green" —
state-graph tests, cargo tests, linting, whatever the project needs. The merge
guard just runs missouri.
