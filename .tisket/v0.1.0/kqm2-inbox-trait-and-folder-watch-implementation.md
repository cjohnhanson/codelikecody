---
title: "Inbox trait and folder-watch implementation"
status: in_progress
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:20:00Z
updated: "2026-03-11T02:22:27Z"
---

Define Inbox trait in clc-sdk: poll for items, return structured messages. First implementation is folder-watch — monitors a directory, picks up new files, yields them as inbox items. Files are moved or marked after processing to avoid re-reads.

Folder-based inbox is testable with missouri: initial state has files in inbox dir, assert they get processed.

Depends on: nothing (trait design is standalone)
Blocks: admin loop, clc up

## Scratch Notes

### Design
- `Inbox` trait: `fn poll(&mut self) -> Result<Vec<InboxItem>, InboxError>`
- `InboxItem`: concrete struct with `content()` and `source()` methods
- `FolderInbox`: polls a dir, moves files to `.processed/` subdir after reading
- Only top-level files processed; subdirs ignored
- Subsequent `poll()` calls return new files only (moved files don't reappear)

### Files
- `clc-sdk/src/inbox.rs` — trait + FolderInbox implementation (to create)
- `clc-sdk/tests/inbox.rs` — integration tests (written, compiles red)
- `clc-sdk/Cargo.toml` — added `tempfile` dev-dependency

### Status
- Tests written (8 tests), fail to compile — red phase
- Next: implement `clc_sdk::inbox` module with `Inbox` trait, `InboxItem`, `FolderInbox`

### Missouri note
- Issue mentions missouri testing, but this is library code; Rust integration tests are appropriate
- Missouri testing of inbox behavior awaits admin loop / `clc up` CLI commands
