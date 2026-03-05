---
title: "clc done should not close tisket when worker completes only a sub-scope of the task"
status: todo
priority: 3
assignee:
labels: []
depends_on: []
created: "2026-03-05T03:20:36Z"
updated: "2026-03-05T03:20:36Z"
---

When a worker is dispatched for a task with a broad scope (e.g. "research and build tap-socrata-rs"), `clc done` closes the entire tisket even when only part of the scope was completed. Observed: a research-only worker couldn't write code due to phase gates, requested a permission to skip phases, finalized with `clc done`, and closed the tisket — leaving the build phase undone with no record.

The tisket closure should reflect whether the full scope was actually delivered, not just whether the worker is finished. A worker that completed only research on a "research and build" task should not be able to close the tisket.

Possible approaches:
- `clc done` warns if no implementation files changed on a tisket whose title implies building something
- Add a `clc park` command — finalize the worker without closing the tisket, leaving it in `todo` for a follow-up
- Let workers mark partial completion explicitly, leaving tisket open for remaining work
