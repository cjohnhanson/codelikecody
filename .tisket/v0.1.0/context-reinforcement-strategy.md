---
title: "Context reinforcement strategy"
status: discovery
priority:
assignee:
labels: [research]
depends_on: []
created: "2026-02-24T04:49:45Z"
updated: "2026-02-24T04:49:45Z"
---

Research optimal strategies for reinforcing workflow context throughout a coding
agent session, not just at SessionStart.

## Questions to answer

- What does the research say about "lost in the middle" and periodic reinforcement
  for LLM context windows?
- What's the right cadence — every Nth UserPromptSubmit? Every Nth tool call?
  Adaptive based on session length?
- Should reinforcement be event-driven (inject on state changes like phase
  transitions, test results) or time/count-based?
- What's the minimal effective payload — full phase description or just
  "phase: implementing"?
- Does injecting on UserPromptSubmit vs PreToolUse vs PostToolUse matter for
  attention/salience?

## Implementation considerations

- Hook already receives `session_id` — per-session counters in `.clc/` could
  track invocation counts
- Cadence could be configurable in `.clc/config.yml`
- Early-session reinforcement may matter more than late-session (agent is still
  orienting)
- Context injection has a cost — tokens in the window that could be used for
  actual work
