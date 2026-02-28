---
title: "Extract claude-code crate for CC protocol and session management"
status: done
priority:
assignee:
labels: [architecture, refactor]
depends_on: []
created: 2026-02-27T13:07:41Z
updated: "2026-02-28T05:58:50Z"
---

Move the stream-json NDJSON types from clc-sdk/src/stream.rs into a new
claude-code crate (protocol module). Add a session module with a typed process
builder for spawning Claude Code with piped stdio.

clc-sdk re-exports claude_code::protocol. WorktreeWorkspace becomes a thin
wrapper around claude_code::Session.
