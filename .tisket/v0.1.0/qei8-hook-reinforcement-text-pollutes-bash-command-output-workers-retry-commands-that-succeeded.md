---
title: "fix: hook reinforcement text pollutes Bash command output — workers retry commands that succeeded"
status: todo
priority: 2
assignee:
labels: [clc, hooks, worker-ergonomics, auto]
depends_on: []
created: 2026-03-26T23:48:00Z
updated: "2026-04-03T18:33:43Z"
---

## Problem

When a worker runs a Bash command (e.g., `clc status set tests-written`),
the PostToolUse or UserPromptSubmit hook fires and injects reinforcement
text into the response. This context text gets concatenated with the
command's actual stdout, so the worker sees a wall of hook-injected
prime text instead of clean command output.

The worker interprets this as a failed or unusual command and retries
with different approaches — `cd` prefixes, env var overrides, output
redirects. In the 3pui worker session, `clc status set tests-written`
was attempted five different ways before the worker moved on, even
though the first invocation succeeded.

The consequence is wasted tokens and context on commands that already
worked, plus potential for workers to enter retry loops on perfectly
functional commands.

## Open Questions

- Is the reinforcement text being injected on PostToolUse (after the
  Bash command), on UserPromptSubmit (before the next prompt), or both?
- Should the reinforcement text go into a separate response field that
  Claude Code surfaces differently from tool output?
- Does Claude Code's hook response schema support a way to attach
  context without polluting the tool result?

## Why It Matters

Every dispatched worker hits this. Phase transitions, git commands, cargo
builds — any Bash command gets reinforcement text mixed into its output.
Workers waste context retrying, and in the worst case could enter loops
trying to "fix" commands that aren't broken.
