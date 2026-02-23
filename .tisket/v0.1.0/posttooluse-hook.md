---
title: "PostToolUse hook"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init, status-transitions, missouri-integration, event-system-and-agent-adapter]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

PostToolUse hook fires after a tool runs. Can auto-detect state changes — e.g.,
after missouri tests run via Bash, detect whether they passed or failed and
update clc state accordingly.

## Missouri tests

State: worktree-implementing (missouri tests exist)
Assertions:
- After a Bash tool use that ran missouri tests (detectable from command/output),
  `.clc/state` reflects updated test status
- Non-missouri Bash commands don't trigger state updates
- PostToolUse with non-Bash tools passes through
