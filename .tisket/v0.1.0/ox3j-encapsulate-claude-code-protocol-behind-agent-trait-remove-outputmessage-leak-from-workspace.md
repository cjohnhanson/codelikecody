---
title: "Encapsulate claude-code protocol behind Agent trait — remove OutputMessage leak from Workspace"
status: in_progress
priority: 2
assignee: coord-encapsulate
labels: [agent, architecture, clc-up-target]
depends_on: []
created: 2026-03-24T13:07:08Z
updated: "2026-03-25T01:52:52Z"
---

The Workspace trait returns `Vec<claude_code::protocol::OutputMessage>` from
`recv_output()`. This forces every workspace implementation to understand
Claude Code's NDJSON protocol. An agent swap (codex, aider, goose) would
require changing the Workspace trait — the opposite of what the Agent
abstraction is for.

## Problem

`clc_sdk::workspace::Workspace::recv_output()` returns
`Vec<claude_code::protocol::OutputMessage>`. The `OutputMessage` type
includes Claude-specific variants: `Assistant`, `System`, `Result` with
`permission_denials`. The `WorktreeWorkspace` parses Claude's NDJSON in
`recv_output()` and extracts denials and completion status from
Claude-specific message shapes.

The Workspace should be agent-agnostic: stdio pipes, process lifecycle,
file management. The Agent should own its protocol — how to parse its
output, how to detect completion, how to extract permission denials.

## Acceptance criteria

1. `Workspace::recv_output()` returns an agent-agnostic type (raw lines,
   or a generic `AgentMessage` enum defined on the Agent trait)
2. `Agent` trait gains methods for parsing its own output format:
   - `parse_output(line: &str) -> Option<AgentEvent>` where `AgentEvent`
     covers completion, failure, permission denial, text output, tool use
3. `WorktreeWorkspace` and `SSHWorkspace` read raw stdout and delegate
   parsing to the Agent
4. No `claude_code::protocol` imports outside of `ClaudeCodeAgent`
5. Existing coordinator/supervisor behavior unchanged — they work with
   `AgentEvent` instead of `OutputMessage`

## Files to change

- `clc-sdk/src/workspace.rs` — trait definition, remove OutputMessage
- `clc-sdk/src/agent.rs` — add AgentEvent, parse methods
- `clc/src/workspace.rs` — WorktreeWorkspace delegates parsing to Agent
- `clc/src/ssh_workspace.rs` — same
- `clc/src/coordinate.rs` — update to use AgentEvent
- `clc/src/worker.rs` — output display code

## Scratch Notes
