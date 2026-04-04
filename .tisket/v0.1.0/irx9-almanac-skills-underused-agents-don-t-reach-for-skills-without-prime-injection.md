---
title: "almanac skills go unused despite prime injection — context decay problem"
status: discovery
priority:
assignee:
labels: [enhancement, almanac, context-lifecycle]
depends_on: []
created: 2026-04-04T12:54:12Z
updated: "2026-04-04T13:07:11Z"
---

## Problem

Agents don't proactively reach for almanac skills during work even
though the skill index IS injected into prime text at session start.
The injection exists in `clc/src/skills.rs` — it includes the full
`almanac list` output (name + description for every skill) and
explicit guidance about when to load skills.

Despite this, agents working on code review don't load
code-review-eval. Agents writing tests don't load testing-strategy.
Agents writing tiskets don't load tisket-writing. The skills exist,
the index is in context, and agents still don't use them.

## What already exists

- `skills.rs` injects an "Almanac (skills)" section into prime text
- The section includes: commands (list/show/search), when-to-load
  guidance, and the full skill index with descriptions
- `status_basic()` includes a one-liner ("almanac: N skills from M
  sources") in UserPromptSubmit reinforcement

## Open Questions

- Is the prime text being pushed out of effective context window as
  conversations grow? Does reinforcement need to include the skill
  index, not just the count?
- Is the "when to load" guidance too passive? Should it be more
  directive? ("You MUST load the relevant skill before...")
- Would nudges help? Post-tool-use reminders like "You're editing
  test files. Did you load a testing skill?"
- Is the skill list too long to scan? Would categorization or
  shorter descriptions improve uptake?
- Is the problem that `status_basic()` only says "almanac: 18 skills
  from 2 sources" without listing them? The count doesn't remind the
  agent what's available.

## Why It Matters

Skills represent curated methodology. If agents don't use them, work
quality depends entirely on the model's baseline behavior, which is
exactly what the skills are designed to improve on.
