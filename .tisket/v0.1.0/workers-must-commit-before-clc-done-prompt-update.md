---
title: "Workers must commit before clc done: prompt update"
status: in_progress
priority:
assignee:
labels: [clc, prompt]
depends_on: []
created: 2026-03-01T13:15:48Z
updated: "2026-03-01T16:51:50Z"
---

The worker-cleanup worker called `clc done` without committing its implementation. The tisket closed, the branch finalized, but all functional code was left as uncommitted working tree changes. If merged, it would have been an empty delivery.

The clean-working-tree guard (branch `clc-done-must-verify-clean-working-tree-before-finalizing`) prevents this mechanically, but the prime text / session context should also tell workers explicitly: stage and commit your work before running `clc done`.

This is a prompt content change — the SessionStart hook context or the reinforcement text should include a clear instruction like "commit all implementation changes before finalizing."
