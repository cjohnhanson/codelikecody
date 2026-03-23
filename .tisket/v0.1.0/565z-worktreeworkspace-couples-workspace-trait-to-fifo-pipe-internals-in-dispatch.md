---
title: "WorktreeWorkspace couples Workspace trait to FIFO pipe internals in dispatch"
status: todo
priority:
assignee:
labels: [clc, clc-sdk, architecture]
depends_on: []
created: "2026-03-23T03:11:52Z"
updated: "2026-03-23T03:11:52Z"
---

## Problem

The `Workspace` trait in clc-sdk should be an abstract interface over workspace lifecycle — start, send, recv, stop — with no opinion on transport. Instead, `WorktreeWorkspace` in `clc/src/workspace.rs` is hardcoded to the FIFO pipe infrastructure: `send_message` writes to `stdin.pipe` via `dispatch::send_prompt`, `recv_output` reads from `stdout.jsonl` with a file cursor, and `start` calls `dispatch::spawn_agent_process` which creates named pipes via `mkfifo`. The trait implementation cannot be swapped for a different transport (Docker, SSH, TCP) without rewriting the entire struct, which defeats the purpose of having the `Workspace` abstraction in clc-sdk at all.

## Open Questions

- Should transport details (pipe paths, cursor tracking) be injected into `WorktreeWorkspace` rather than hardcoded?
- Is the `Workspace` trait meant to support non-FIFO backends (the `DockerWorkspace` tisket suggests yes), and if so, what's the right abstraction for the communication channel?
- Does the tight coupling to `dispatch::send_prompt` and `dispatch::spawn_agent_process` also leak into coordinator code?

## Why It Matters

The `DockerWorkspace` implementation (tracked in a separate tisket) cannot reuse any of this code — it will need to reimplement the entire `Workspace` trait from scratch. The coupling means the trait isn't actually providing the polymorphism it promises.
