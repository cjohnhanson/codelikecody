---
title: "Reconsider coordinator-facing context -- what information is most salient for monitoring workers"
status: discovery
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T20:30:00Z
updated: "2026-03-01T16:52:20Z"
---

## Problem

The information surfaced by clc for worker monitoring was built incrementally without stepping back to ask: what does a coordinator (human or agent) actually need to know?

Current state of surfaces:
- `clc workers` shows: name, alive/dead, pid, line count, last NDJSON message type
- `clc worker <id> check` shows: last N lines of raw NDJSON
- `clc worker <id> log` shows: last N lines of stdout

What's missing or misleading:
- "user" as a status means nothing to a coordinator -- it's a protocol detail
- Line count grows but doesn't indicate progress
- No phase, no elapsed time, no indication of what the worker is actually doing
- No distinction between "thinking for 30s" and "stuck for 10 minutes"
- No summary of what the worker has accomplished so far
- The coordinator has to grep raw NDJSON to understand state

## Discovery needed

This is a design question, not just a formatting fix. The worker-visibility tisket covers the specific output improvements. This tisket is about stepping back and asking:

1. **What questions does a coordinator ask?**
   - Is the worker alive?
   - What phase is it in?
   - Is it making progress?
   - Is it stuck?
   - What has it accomplished?
   - Should I intervene?

2. **What information answers those questions?**
   - Phase (from .clc/state)
   - Time since last output line
   - Last meaningful action (tool name + target, not just event type)
   - Cumulative summary: files edited, tests run, commits made
   - Error rate: how many tool failures vs successes

3. **What format is most useful?**
   - For a human coordinator: readable summary
   - For an agent coordinator: structured data it can reason about
   - Both? Different commands?

4. **What about the NDJSON stream itself?**
   - Is there useful structure we're ignoring?
   - Should worker check parse and summarize rather than dumping raw lines?
   - Should there be a "worker summary" that distills the whole session?

## Related tiskets

- worker-visibility-show-phase-last-action-and-elapsed-time-in-clc-workers (specific output fixes)
- worker-failure-modes-observed-during-coordinator-operation (what goes wrong)
