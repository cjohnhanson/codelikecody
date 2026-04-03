---
title: "clc missouri detect() ignores workspace members — always reports no tests"
status: todo
priority: 3
assignee:
labels: [clc, missouri, auto]
depends_on: []
created: 2026-03-19T03:27:51Z
updated: "2026-04-03T18:33:12Z"
---

`clc/src/missouri.rs` `detect()` (line ~87) looks for a single `tests/missouri/` directory at the project root. It never reads `missouri.yml` for workspace members. Projects using workspace mode — like this one, with four members (`clc/tests/missouri`, `clc-api/tests/missouri`, `tisket/tests/missouri`, `missouri/tests/missouri`) — always get "missouri: no tests" in hook context injection.

The missouri CLI itself handles this correctly: `load_workspace_members()` in `missouri/src/graph.rs` reads `missouri.yml`, checks for a `members` field, and iterates over each member's test directory. `detect()` should do the same — check for workspace members first, aggregate path/state counts across members, fall back to single-directory discovery when no workspace config exists.

`status_basic()` (line ~261) can then report the aggregate counts instead of unconditionally returning "no tests".
