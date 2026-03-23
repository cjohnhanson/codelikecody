---
title: "Loop wrapper: replace stop hooks with resume-loop lifecycle management"
status: discovery
priority: 2
assignee:
labels: [clc, architecture]
depends_on: [am9k-agent-trait-extract-claude-specific-code-from-workspace-into-agent-abstraction, io9i-coordinationbackend-trait-with-postgres-implementation-via-seaorm]
created: 2026-03-22T22:02:19Z
updated: "2026-03-23T02:12:47Z"
---

## Problem

Worker lifecycle should be managed by a loop wrapper that handles resume, retry, and graceful shutdown — the worker runs until its task is complete, with the loop managing restarts after permission grants, context compaction, or transient failures.

Currently, worker stop behavior is handled by the `check_stop` function in `guard.rs`, which blocks the Stop event if the phase isn't `Done`, `ReviewRequested`, or `Reviewed`. Resume is handled manually via `clc worker <id> resume`, which sends a new prompt through the FIFO pipe (`dispatch::send_prompt`). There is no loop wrapper — the worker is a single Claude Code process. If it exits (permission denial, crash, context limit), human intervention is required to resume it.

Without a loop wrapper, workers that hit permission denials sit idle until a coordinator or human manually resumes them. Workers that hit context limits simply die. The coordinator must poll for these conditions rather than being notified, and resumption requires reconstructing context from scratch.

## Recent Changes (io9i)

The io9i landing introduced two layers of resume/restart:

- **coordinator_loop.rs** implements external resume via poll-based monitoring — it watches worker status and auto-restarts workers that have stopped or failed. This is the "loop wrapper" described above, implemented at the coordinator level.
- **worker.rs** gained a `supervise()` function for CLI-level auto-resume, providing process-level restart when a worker exits.

The original problem (human intervention required on every unexpected exit) is substantially addressed for the common cases. The remaining question is whether workers should also self-loop internally — managing their own context (compaction, scratch note checkpointing) and recovering from transient failures without requiring a full process restart.

## Open Questions

- Should the loop wrapper live in clc (wrapping the dispatch spawn) or in a separate daemon process?
- How does the loop wrapper interact with the Workspace trait — is it part of the trait implementation or a layer above it?
- What's the resume protocol: replay the original prompt, inject a "you were resumed because X" message, or read from scratch notes?
- How does this interact with the CoordinationBackend trait (dependency: io9i) and the Agent trait extraction (dependency: am9k)?

## Why It Matters

Manual resume is the bottleneck in autonomous operation. Every time a worker stops unexpectedly, throughput drops to zero for that task until a human notices and acts. A loop wrapper turns workers from fragile single-shot processes into resilient long-running agents.
