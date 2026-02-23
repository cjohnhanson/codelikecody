---
title: "Worktree guard"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init, git-state-detection, event-system-and-agent-adapter]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

On main branch, only read operations are allowed. Everything else gets blocked.
Allowed tools: Read, Glob, Grep. Everything else denied with a message about
creating a worktree.

## Missouri tests

State: initialized-on-main (clc init'd, git repo on main branch)

Assertions (pipe hook event JSON to `clc hook`, check exit codes):
- PreToolUse with tool_name=Read → exit 0 (passthrough)
- PreToolUse with tool_name=Glob → exit 0
- PreToolUse with tool_name=Grep → exit 0
- PreToolUse with tool_name=Edit → exit 2 (blocked)
- PreToolUse with tool_name=Write → exit 2 (blocked)
- PreToolUse with tool_name=Bash → exit 2 (blocked)
- PreToolUse with tool_name=Task → exit 2 (blocked)
- Block message contains guidance about worktree workflow
