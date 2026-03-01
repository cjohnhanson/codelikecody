---
title: "Coordinator has no inbound communication channel — user cannot talk to a running coordinator"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-01T15:43:20Z
updated: "2026-03-01T16:38:08Z"
---

The coordinator was designed to be the user's interface for managing workers — the user talks to the coordinator, the coordinator handles dispatch and monitoring. But the coordinator has no inbound communication channel.

Workers have `stdin.pipe` because the coordinator creates it at spawn time. Nothing creates an equivalent channel for the coordinator itself. An interactive session has no way to reach a running coordinator, and a running coordinator has no way to receive user input.

## Approach

The coordinator should have the same interface as a worker — `stdin.pipe` for input, `stdout.jsonl` for output. The user's interactive session talks to the coordinator via the existing `clc worker <id> send` / `clc worker <id> check` commands. One level of misdirection: the user talks to an interactive session, the interactive session talks to the coordinator as if it were a worker.

The coordinator is just a worker that happens to manage other workers.
