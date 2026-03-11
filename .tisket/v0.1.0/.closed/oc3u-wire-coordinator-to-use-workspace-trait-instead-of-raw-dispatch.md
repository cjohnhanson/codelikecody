---
title: "Wire coordinator to use Workspace trait instead of raw dispatch"
status: done
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:09:47Z
updated: "2026-03-11T02:48:31Z"
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

### Files consulted
- clc-sdk/src/workspace.rs - Workspace trait def (start, send_message, recv_output, status, stop)
- clc/src/workspace.rs - WorktreeWorkspace (uses Session from claude-code, piped stdio)
- clc/src/coordinate.rs - coordinate() calls dispatch::spawn_worker_process directly (the target)
- clc/src/dispatch.rs - spawn_worker_process uses named FIFOs + files (detached background process)
- clc/src/worker.rs - working_dir_for, worker_dir_for, COORDINATOR_ID handling
- claude-code/src/session.rs - Session uses piped stdio (different from FIFO approach)

### Architecture understanding
- Coordinator agent runs on trunk (project_dir), not in a worktree
- Worker agents run in .worktrees/<id>/
- Current infra: spawn_worker_process creates stdin.pipe (FIFO), stdout.jsonl, stderr.log, pid files
- WorktreeWorkspace currently uses Session (piped stdio) - different from FIFO approach
- "Same pipes, same output" means we must keep FIFO infrastructure
- So WorktreeWorkspace needs to be updated to use spawn_worker_process internally (not Session)
- working_dir for coordinator is project_dir (trunk); WorktreeWorkspace hardcodes .worktrees/<id>

### Plan
1. Update WorktreeWorkspace to use spawn_worker_process + FIFO infrastructure (not Session)
2. Make working dir + worker dir configurable (coord needs project_dir + .clc/worker/)
3. Update coordinate() to create a WorktreeWorkspace (or coord variant) and call start()
4. coordinate() becomes generic over impl Workspace via a new coordinate_with() internal function

### Test strategy
- Add coordinate_with() function that takes a workspace factory
- Write unit tests using MockWorkspace to verify start() is called with correct config
- Test setup requires a minimal git repo + tisket (can use Command::new("git") in tests)
- Tests will fail to compile/run until coordinate_with() is implemented

### Current status
- Phase: tests-unwritten (about to write tests)
- Next: Write tests in coordinate.rs for the coordinate_with() API
