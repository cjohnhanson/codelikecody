---
title: "add unit tests for tisket Status methods (is_active, is_terminal, is_pickable)"
status: todo
priority: 3
assignee:
labels: [clc-up-target]
depends_on: []
created: "2026-03-27T02:27:20Z"
updated: "2026-03-27T02:27:20Z"
---

`tisket/src/issue.rs` has three methods on `Status` with zero test coverage:

- `is_active()` — returns true for `InProgress`
- `is_terminal()` — returns true for `Done` and `Cancelled`
- `is_pickable()` — returns true for `Todo`, `Blocked`, `Paused`

Add a `#[cfg(test)] mod tests` block to `tisket/src/issue.rs` with tests
that verify each method returns the correct value for every `Status` variant.

## Done when

- `cargo test -p tisket` passes with new tests
- Every `Status` variant is tested against each of the three methods
- No other code changes needed — this is pure test addition

## Scratch Notes
