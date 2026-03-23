---
title: "missouri state fixtures invisible to cargo test — 82 acceptance tests require manual invocation"
status: todo
priority:
assignee:
labels: [missouri, testing, ci]
depends_on: []
created: "2026-03-23T03:12:04Z"
updated: "2026-03-23T03:12:04Z"
---

## Problem

1. The 82 missouri state fixtures in `clc/tests/missouri/` should run as part of the normal test workflow so that `cargo test --workspace` catches regressions in clc's behavior against known filesystem states.
2. `clc/tests/` contains only the `missouri/` directory with fixture state directories — there are no `.rs` integration test files. `cargo test` does not discover or execute missouri fixtures; they require manual invocation via the `missouri` binary. Nothing in the cargo test configuration references them.
3. A developer or CI pipeline running `cargo test --workspace` gets a clean pass while 82 acceptance tests sit unexecuted. Regressions in clc behavior that the fixtures would catch go undetected until someone remembers to run missouri manually.

## Open Questions

- Should missouri fixtures be wrapped in a `#[test]` harness (a Rust integration test that shells out to `missouri run`), or should CI run missouri separately as a post-test step?
- Does the missouri binary need to be built first (`cargo build` dependency), and how should that be expressed in the test setup?
- Are all 82 fixtures currently passing, or are some already broken and just invisible?

## Why It Matters

82 acceptance tests that don't run automatically aren't tests — they're documentation that rots. The whole point of missouri is to catch behavior regressions against known states. If `cargo test` doesn't trigger them, the safety net has a hole exactly where it matters most.
