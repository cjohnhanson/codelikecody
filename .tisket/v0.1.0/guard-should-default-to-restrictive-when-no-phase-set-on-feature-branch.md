---
title: "Guard should default to restrictive when no phase set on feature branch"
status: backlog
priority: 2
assignee:
labels: [clc, guard]
depends_on: []
created: "2026-02-26T04:13:09Z"
updated: "2026-02-26T04:13:09Z"
---

`guard.rs` lines 76-78 treat "no phase on feature branch" as permissive:

```rust
let Some(current_phase) = phase else {
    // No phase set — allow everything (pre-phase workflow).
    return Response::Passthrough;
};
```

This bypasses the entire phase system for worktrees not created via `clc pickup`.
The safe default is restrictive — if there's no phase, assume `tests-unwritten`
behavior (only test file edits allowed). Same for `check_stop`: no phase on a
feature branch should block stop, not allow it.

This is the safety net. SessionStart auto-setting phase is the primary mechanism,
but the guard should be independently safe.
