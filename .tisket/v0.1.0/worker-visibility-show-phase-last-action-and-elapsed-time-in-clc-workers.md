---
title: "Worker visibility: show phase, last action, and elapsed time in clc workers"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-02-28T06:50:06Z"
updated: "2026-02-28T06:50:06Z"
---

Current `clc workers` output shows pid, line count, and last event type. That's
not enough to know what a worker is actually doing. "No new activity" followed
by speculation about thinking blocks is unacceptable operational visibility.

## Problems

1. `clc workers` shows `working` for processes that finished hours ago (the
   completed-but-not-reaped problem, separate tisket exists)
2. No phase information — have to manually cat `.clc/state` in the worktree
3. No elapsed time since last activity — can't distinguish "thinking for 30s"
   from "stuck for 10 minutes"
4. `check` shows raw NDJSON event types (tool, user, thinking) — not
   human-readable summaries of what's happening
5. No way to see if a long pause is thinking vs compiling vs stuck

## Desired state

`clc workers` should show at minimum:
- Phase (from `.clc/state` in the worktree)
- Time since last NDJSON line was written
- Whether the process is actually alive (not just listed)

`clc worker <id> check` should show:
- Current phase
- Last meaningful action (not just event type — what tool, what file)
- Elapsed time since last output
- Total runtime

## Notes

The raw NDJSON has the information — it's a presentation problem. The check
command already does cursor-based reading; it just needs to parse more of the
content and summarize it.
