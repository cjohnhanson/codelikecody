---
title: "PostToolUse hook"
status: done
priority:
assignee:
labels: [feature]
depends_on: []
created: "2026-02-23T02:23:25Z"
updated: "2026-02-25T04:35:00Z"
---

PostToolUse hook fires after a tool completes. Primary use: inject contextual
nudges after tool uses based on current state.

Original approach (parsing Bash output to auto-detect missouri test results) was
scrapped — too fragile. Missouri will persist its own results instead; clc reads
the cached state.

## Possible uses

- After Edit/Write tools: nudge about running tests if results are stale
- After Bash: check if missouri results file was updated, inject pass/fail summary
- Phase-aware reminders (e.g., "still in tests-unwritten, don't implement yet")

## Depends on

- Missouri persisted results (timestamp-based caching)
- Context reinforcement strategy (what to inject and when)
