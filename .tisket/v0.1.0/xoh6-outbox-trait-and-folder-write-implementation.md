---
title: "Outbox trait and folder-write implementation"
status: in_progress
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:20:07Z
updated: "2026-03-11T02:22:27Z"
---

Define Outbox trait in clc-sdk: accept structured items, write them somewhere. First implementation is folder-write — writes files to a configured directory. Each outbox item becomes a file (markdown, JSON, whatever the admin produces).

Folder-based outbox is testable with missouri: transition produces output, assert files appear in outbox dir.

Depends on: nothing (trait design is standalone)
Blocks: admin loop, clc up

## Scratch Notes
