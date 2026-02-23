---
title: "Event system and agent adapter"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: []
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

clc defines its own event model (about-to-edit, session-starting, agent-stopping,
etc.) independent of any specific coding agent. Agent adapters translate between
the agent's hook system and clc events. Claude Code is the first adapter.

This keeps coupling minimal — core logic never knows which agent it's talking to.

## Missouri tests

State: bare-project
Transition: `clc init --agent claude-code` → initialized-claude
State: initialized-claude (`.claude/settings.local.json` with Claude Code hooks)

Transition: `clc init --agent generic` → initialized-generic
State: initialized-generic (some other hook mechanism, TBD)

Assertions:
- Claude adapter produces correct `.claude/settings.local.json` structure
- Same clc event input produces same logical output regardless of which adapter
  initialized the project (tested by piping event JSON to `clc hook`)
