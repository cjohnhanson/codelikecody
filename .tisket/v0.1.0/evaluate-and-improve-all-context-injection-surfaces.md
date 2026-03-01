---
title: "Evaluate and improve all context injection surfaces"
status: discovery
priority:
assignee:
labels: [clc]
depends_on:
  - context-compaction-drops-operational-knowledge-workers-forget-project-tooling
  - prime-text-should-mandate-tdd-independently-of-phase-enforcement
  - worker-visibility-show-phase-last-action-and-elapsed-time-in-clc-workers
  - reconsider-coordinator-facing-context-what-information-is-most-salient-for-monitoring-workers
  - contextual-skill-management-and-preferences-system-for-clc
created: "2026-02-28T20:35:00Z"
updated: "2026-02-28T20:35:00Z"
---

Epic. clc injects context at multiple surfaces -- prime text, reinforcement, post-tool nudges, stop messages, worker status. This epic covers stepping back and evaluating whether the right information is being sent at each surface, whether it is sufficient, and whether the overall context budget is being spent well.

## Context surfaces

- **SessionStart prime** -- the big injection. Missouri section, tisket section, workflow description, TDD mandate, commit discipline, etc. Currently ~200 lines.
- **UserPromptSubmit reinforcement** -- lean status line. Tisket status, missouri status, phase.
- **PostToolUse nudge** -- "phase: implementing -- run tests before advancing" after file edits.
- **Stop hook message** -- blocks with phase-specific error when stopping too early.
- **clc workers / clc worker check** -- coordinator-facing status of dispatched workers.
- **CLAUDE.md** -- project instructions, currently flat and not contextual.

## The question

For each surface: is the content the most salient information for the recipient at that moment? Are there gaps (like missouri not saying how to run tests)? Is there noise (context that wastes tokens without driving behavior)?
