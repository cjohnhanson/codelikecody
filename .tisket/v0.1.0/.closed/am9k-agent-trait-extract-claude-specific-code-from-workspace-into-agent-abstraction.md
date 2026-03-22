---
title: "Agent trait: extract claude-specific code from workspace into Agent abstraction"
status: done
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-22T22:01:51Z
updated: "2026-03-22T23:48:41Z"
---

Extract claude-specific code from WorktreeWorkspace and dispatch.rs into an Agent trait in clc-sdk. ClaudeCodeAgent implements it by spawning claude with stream-json.

Trait surface: start session, send message, receive output, stop. The workspace holds an Agent — it doesn't know what agent is running.

Currently Command::new("claude") is hardcoded in dispatch.rs:129 and worker.rs:309. The protocol types in claude_code::protocol are already in clc-sdk.

No new behavior — pure refactor. Existing tests should pass unchanged.
