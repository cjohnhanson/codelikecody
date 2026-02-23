---
title: "UserPromptSubmit hook"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init, status-transitions, event-system-and-agent-adapter]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

UserPromptSubmit hook intercepts every user prompt and injects phase-appropriate
reminders. Unlike SessionStart (fires once), this fires on every prompt and can
reinforce the current workflow phase.

## Missouri tests

Assertions (pipe UserPromptSubmit JSON to `clc hook` in various states):
- In tests-unwritten phase: output includes reminder about missouri tests
- In implementing phase: output includes implementation context
- On main: output includes reminder about worktree workflow
- Output is valid JSON or empty (passthrough) depending on state
