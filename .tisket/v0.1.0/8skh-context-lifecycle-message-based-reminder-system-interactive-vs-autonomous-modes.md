---
title: "context lifecycle: message-based reminder system, interactive vs autonomous modes"
status: discovery
priority:
assignee:
labels: [architecture, clc]
depends_on: []
created: "2026-04-04T13:06:49Z"
updated: "2026-04-04T13:06:49Z"
---

## Problem

Prime text injects context at session start, but that context decays
as conversations grow. Information migrates from the top of the context
window to the middle, where attention drops (the U-shaped attention
curve). Skills, phase guidance, and project context all suffer from
this. The current clc remind is time-based (cron-style), which doesn't
map to how context actually decays — decay is a function of messages
and tool calls, not wall clock time.

Additionally, the hook system treats interactive sessions (human
driving) and autonomous sessions (agent working alone) as roughly
equivalent. These need different context management: interactive
sessions need less guardrailing but more collaborative context;
autonomous sessions need more reinforcement and stricter enforcement.

## Design direction

Three mechanisms, each serving a different purpose:

### 1. `clc cron` (current `clc remind`, renamed)

Time-based periodic wake-up. "Every 30 minutes, check worker health."
Fires even when the agent is idle. Equivalent to Claude Code's built-in
cron, intentionally redundant for agent-agnosticism. The agent is
the actor.

### 2. `clc remind` (new, message-counter-based)

Hook-driven context re-injection. Different sections of prime text
decompose into independently refreshable units, each with a configurable
cadence:

- Skills index: every ~20 messages
- Phase guidance: every ~5 messages
- Tisket context: every ~10 messages
- Custom sections: per-workflow config

The hook system tracks a message counter. When a section's counter
threshold is hit, it re-injects that section as a system reminder.
No timer, no cron — just "it's been N interactions since you last
saw this."

Agents can also self-schedule: `clc remind schedule "check the CI
run" --after 10` inserts a one-shot reminder 10 messages into the
agent's future context. The agent reaches forward in time.

### 3. Interactive vs autonomous modes

The workflow config expresses the session mode. Interactive sessions
get different cadences, different sections, and different enforcement
levels than autonomous sessions. The hook system reads the mode and
adjusts behavior.

All three mechanisms are configurable as part of a workflow definition
alongside phases and permissions.

## Open Questions

- How does the hook track message count? File-based counter in
  .clc/state? Environment variable?
- What's the right granularity for "sections"? Is it the ClcTool
  trait's prime() output, or something more fine-grained?
- How does interactive vs autonomous mode get set? Detected (is there
  a TTY?) or declared (workflow config)?
- Should remind cadences be absolute (every N messages) or adaptive
  (more frequent when the agent seems to be drifting)?
- How does this interact with Claude Code's built-in context
  compression? If the system compresses earlier messages, do the
  re-injected reminders compensate?

## Why It Matters

The entire enforcement model depends on agents seeing and acting on
context. If prime text decays and isn't refreshed, the phase system's
instructions become invisible, skills go unused, and project context
is forgotten. Context lifecycle management is load-bearing
infrastructure for everything else clc does.
