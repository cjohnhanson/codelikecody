---
title: "Wire coordinator to use Workspace trait instead of raw dispatch"
status: in_progress
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:09:47Z
updated: "2026-03-11T02:22:26Z"
---

The Workspace trait exists in clc-sdk (start, send_message, recv_output, status, stop) and WorktreeWorkspace implements it in clc/src/workspace.rs. But coordinate.rs bypasses it entirely - calls dispatch::spawn_worker_process directly and manages workers through the old pipe/pid infrastructure.

Wire the coordinator loop to create and manage WorktreeWorkspace instances instead of calling dispatch directly. The coordinator should be generic over impl Workspace so that future backends (Coder, Docker, local LLM) slot in without changing coordinator logic.

Key files:
- clc-sdk/src/workspace.rs - trait definition
- clc/src/workspace.rs - WorktreeWorkspace impl
- clc/src/coordinate.rs - coordinator loop (currently bypasses trait)
- clc/src/dispatch.rs - raw process spawning (should become internal to WorktreeWorkspace)

What done looks like:
- Coordinator creates WorktreeWorkspace instances and calls trait methods
- dispatch::spawn_worker_process becomes an implementation detail of WorktreeWorkspace, not called directly by the coordinator
- No behavioral change - same workers, same pipes, same output. Just properly abstracted.

## Scratch Notes
