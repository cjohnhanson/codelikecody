---
title: "Add missing missouri guard test coverage"
status: in_progress
priority:
assignee:
labels: [testing]
depends_on: []
created: 2026-02-27T15:45:57Z
updated: "2026-02-27T15:46:35Z"
---

The guard system in `clc/src/guard.rs` has several untested code paths in the
missouri e2e suite. These gaps mean regressions could ship without detection.

## Missing coverage

1. **NotebookEdit** — in `FILE_TARGETING_TOOLS` but never tested. Should be blocked
   on trunk and in restricted phases, same as Edit/Write.

2. **CLC_GUARD_OFF escape hatch** — the env var bypass at the top of `evaluate()`
   has no test. If the check logic changes, nothing catches it.

3. **Bash allowlist completeness** — several allowlisted prefixes have no test:
   `cargo fmt --check`, `cargo check`, `cargo build`, `which`, `cat`, `head`,
   `tail`, `wc`, `find`, `tree`, `tisket search`, `tisket issue show`,
   `tisket issue path`.

4. **tests-written phase guard** — same restriction as tests-unwritten (only
   test paths editable) but no Edit/Write block assertions in that state.

5. **red phase guard** — same gap as tests-written.

6. **Bash passthrough on feature branch** — Bash is allowed in all phases on
   feature branches but this is never asserted.

## Where to add

- Trunk guard tests go in `initialized/.missouri/missouri.yml` assertions
- Phase guard tests go in the respective `phase-*/.missouri/missouri.yml` assertions
- New tests for CLC_GUARD_OFF can go in `initialized`
