---
title: "Inbox trait and folder-watch implementation"
status: todo
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:20:00Z
updated: "2026-03-11T02:22:22Z"
---

Define Inbox trait in clc-sdk: poll for items, return structured messages. First implementation is folder-watch — monitors a directory, picks up new files, yields them as inbox items. Files are moved or marked after processing to avoid re-reads.

Folder-based inbox is testable with missouri: initial state has files in inbox dir, assert they get processed.

Depends on: nothing (trait design is standalone)
Blocks: admin loop, clc up
