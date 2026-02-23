---
title: "Phase enforcement"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [worktree-guard, status-transitions]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

In a worktree, what's allowed depends on the current phase:
- tests-unwritten: only edits in tests/missouri/ dir allowed
- tests-written/red: only edits in tests/missouri/ dir allowed
- implementing: all edits in the worktree allowed
- green: only edits in tests/missouri/ (fixing tests) or `clc done`

## Missouri tests

State: worktree-tests-unwritten (phase=tests-unwritten)
Assertions:
- Edit targeting tests/missouri/... → allowed
- Edit targeting src/... → blocked
- Edit targeting Cargo.toml → blocked

State: worktree-implementing (phase=implementing)
Assertions:
- Edit targeting src/... → allowed
- Edit targeting tests/missouri/... → allowed
- Edit targeting Cargo.toml → allowed
