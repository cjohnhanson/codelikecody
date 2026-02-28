---
title: "Interactive worker management CLI"
status: done
priority: 2
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T04:40:16Z
updated: "2026-02-28T05:58:50Z"
---

Replace the batch coordinator model with interactive worker management commands
that let the coordinator session (this Claude Code conversation) dispatch, monitor,
communicate with, and land workers.

## Commands

```
clc dispatch <tisket-id> [--model sonnet] [--budget 5.0]
```
Pickup tisket + spawn claude worker in worktree + return immediately.
Worker runs detached. Sets up named pipe for stdin, NDJSON file for stdout,
PID file. All worker state lives in the worktree.

```
clc workers
```
Scan worktrees for live worker PIDs. Print summary table: id, status
(working/idle/dead), tool calls, last activity, cost.

```
clc worker <id> check
```
Parse NDJSON since last check (cursor-based). Show what happened: tools called,
text produced, errors, whether worker is idle or actively working.

```
clc worker <id> log [--lines 50]
```
Full parsed output, last N lines.

```
clc worker <id> send <message>
```
Write user message JSON to the worker's named pipe.

```
clc worker <id> stop
```
Kill worker process. Leave worktree intact.

```
clc worker <id> raw [--lines 10]
```
Raw NDJSON output for debugging.

```
clc land <id>
```
Stop worker if alive + verify phase done + tisket closed + ff-merge + cleanup
worktree and branch. Subsumes current `clc merge` for worker-managed branches.

## Worker state layout (in worktree)

```
.clc/worker/
  pid           # worker process PID
  stdout.jsonl  # raw NDJSON from claude
  stderr.log    # claude stderr
  stdin.pipe    # named pipe for sending messages
```

## Coordinator state (on trunk)

```
.clc/workers/<id>/
  cursor        # line number of last-read position in stdout.jsonl
```

## Key design points

- NDJSON file is the source of truth for worker history. No derived state db.
- Named pipe gives persistent write end for multi-turn communication.
- Worker stays alive after result message, waiting for next input (stream-json
  behavior). Multi-turn is a first-class capability.
- `dispatch` spawns and detaches — coordinator session stays interactive.
- Cursor is coordinator-side, ephemeral. Lost cursor = full replay, not data loss.
- `coordinate` batch mode can coexist or be retired later.
