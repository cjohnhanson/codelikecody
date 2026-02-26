---
title: "Fix dbt illinois fixture assertions"
status: backlog
priority: 3
assignee:
labels: [missouri, tests]
depends_on: []
created: "2026-02-26T03:57:41Z"
updated: "2026-02-26T03:57:41Z"
---

The `illinois_dbt_flox_passes` test is marked `#[ignore]` with the comment
"dbt fixture assertions broken — pre-existing". Two failures:

1. `dbt-seeded → dbt-ran`: `dbt run` produces a `target/` directory with compiled
   SQL that isn't in the expected `dbt-ran` state. Fix: add the expected `target/`
   tree to the fixture, or use comparator ignores for non-deterministic compiler
   output.

2. `empty → uv-initialized → uv-added`: `uv.lock` drifts across environments/versions.
   Fix: use a comparator that validates structure rather than exact content, or
   ignore the lock file and assert via a command instead.

Once fixed, remove the `#[ignore]` attribute from the test in
`missouri/tests/illinois.rs`.
