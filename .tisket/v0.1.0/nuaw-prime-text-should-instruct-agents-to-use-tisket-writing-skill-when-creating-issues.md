---
title: "feat: prime text should instruct agents to use tisket-writing skill when creating issues"
status: todo
priority: 3
assignee:
labels: [clc, agent-behavior, auto]
depends_on: []
created: 2026-03-26T23:49:59Z
updated: "2026-04-03T18:33:32Z"
---

## Problem

The tisket section of the prime text tells agents how to create issues
(`tisket issue create -p v0.1.0 "title"`) and describes the status
lifecycle. It does not mention the tisket-writing skill or instruct
agents to load it before creating issues.

The result: agents create bare-title issues with no body, no problem
statement, no acceptance criteria. This is exactly what happened across
the original backlog — dozens of empty-body issues that had to be
triaged and rewritten. The tisket-writing skill exists to prevent this,
but agents don't know to use it because the prime text doesn't say to.

## Open Questions

- Should the prime text say "load `almanac show tisket-writing` before
  creating issues" or should the instruction be shorter — just "follow
  the tisket-writing skill" with the assumption the skill is listed in
  the almanac section?
- Should the instruction apply only to `todo` issues (which need
  acceptance criteria) or also to `discovery` issues (which need at
  minimum a problem statement)?
- Should this be in the tisket section of prime text, or in the
  "capturing discovered work" section?

## Why It Matters

Every worker and coordinator creates tiskets during normal operation —
capturing discovered work, filing bugs, scoping follow-ups. Without
the instruction to use the skill, every created issue is a bare title
that requires manual scoping later. The skill exists, it's bundled in
almanac, agents just need to be told to use it.
