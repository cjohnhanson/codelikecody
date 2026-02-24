---
title: "SessionStart context"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init, git-state-detection, event-system-and-agent-adapter]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T03:40:20Z"
---

SessionStart hook injects workflow-aware context. What gets primed depends on
where you are:
- On main: "pick up a tisket, create a worktree"
- In worktree, tests-unwritten: "write missouri tests for this tisket"
- In worktree, implementing: "implement until green"
- In worktree, green: "run clc done"

## Missouri tests

Assertions (pipe SessionStart JSON to `clc hook`, check output):
- On main: output contains additionalContext with worktree workflow guidance
- In worktree with phase=tests-unwritten: context mentions missouri tests
- In worktree with phase=implementing: context mentions implementation
- Output is valid JSON with hookSpecificOutput.additionalContext
