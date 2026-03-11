---
title: "Outbox trait and folder-write implementation"
status: done
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:20:07Z
updated: "2026-03-11T02:50:07Z"
---

Define Outbox trait in clc-sdk: accept structured items, write them somewhere. First implementation is folder-write — writes files to a configured directory. Each outbox item becomes a file (markdown, JSON, whatever the admin produces).

Folder-based outbox is testable with missouri: transition produces output, assert files appear in outbox dir.

Depends on: nothing (trait design is standalone)
Blocks: admin loop, clc up

## Scratch Notes

### Design decisions

- `Outbox` trait goes in `clc-sdk/src/outbox.rs`
- `FolderOutbox` is the first implementation: writes each item as a file to a dir
- Default outbox directory: `.clc/outbox/` (created on first write)
- `OutboxItem` has `name` (filename) and `content` (string)
- CLI command: `clc outbox write <name>` — reads content from stdin
- Writes to `.clc/outbox/<name>` by default

### Test coverage (Missouri)

Path: `initialized` → (clc outbox write item.md) → `outbox-with-item` → (clc outbox write data.json) → `outbox-with-two-items`

States created:
- `clc/tests/missouri/outbox-with-item/` — assertions: file exists, content correct, count=1
- `clc/tests/missouri/outbox-with-two-items/` — assertions: both files exist, count=2
- Transition added to `initialized/.missouri/missouri.yml`

### Implementation plan

1. `clc-sdk/src/outbox.rs` — `Outbox` trait + `FolderOutbox` impl + unit tests
2. `clc/src/cli.rs` — add `Outbox { Write { name } }` subcommand
3. `clc/src/main.rs` — route `clc outbox write <name>` to FolderOutbox

### Files NOT relevant to this feature

- coordinator, permissions, integrate, workers — unrelated
