---
title: "Dispatch smoke test"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T05:14:09Z
updated: "2026-02-28T05:58:50Z"
---

This is a mock tisket for testing the dispatch/worker commands interactively.

## Task

This is a SMOKE TEST for the dispatch system. Do NOT do real work.

1. Add a comment block to the top of `src/main.rs`:

```rust
// DISPATCH SMOKE TEST
// This comment was added by a dispatched worker.
// It should be removed after verifying the dispatch flow works.
```

2. Then remove the comment block you just added.

The task is intentionally trivial — the point is to verify the dispatch,
worker monitoring, and stop commands work end-to-end.
