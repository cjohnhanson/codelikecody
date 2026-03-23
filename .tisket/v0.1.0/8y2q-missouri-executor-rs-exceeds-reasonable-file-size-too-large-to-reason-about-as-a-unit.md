---
title: "missouri executor.rs exceeds reasonable file size — too large to reason about as a unit"
status: discovery
priority:
assignee:
labels: [missouri, maintainability]
depends_on: []
created: 2026-03-23T03:11:53Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

A single source file should be small enough to reason about as a unit — broadly, under 500-800 lines for complex logic. `missouri/src/executor.rs` is 3,196 lines. It contains sandbox detection, nix backend management, mitmdump proxy lifecycle, service orchestration, path environment building, setup phase execution, parallel test execution via rayon, state copying, assertion running, transition execution, and OCI image building — at least 12 distinct responsibilities in one file. The public API surface includes `detect_sandbox`, `build_network_env`, `start_mitmdump_replay`, `start_mitmdump_record`, `start_service`, `run_setup_phase`, `run_all_paths`, and more.

## Open Questions

- What are the natural module boundaries? Sandbox/backend detection, service lifecycle, test execution, and OCI building seem like clear candidates for extraction.
- Does the file's size reflect tight coupling between these responsibilities, or is it just organizational neglect?
- How much of the test code (lines ~2300-3196) is testing internal helpers that would need to become `pub(crate)` after extraction?

## Why It Matters

At 3,196 lines, no one reads this file end-to-end. Bugs hide in the sheer volume. New contributors can't find the function they need. And the file's size makes merge conflicts near-certain when multiple people touch any part of the executor.
