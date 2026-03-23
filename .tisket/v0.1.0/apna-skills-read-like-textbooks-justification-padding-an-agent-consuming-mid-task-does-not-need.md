---
title: "skills read like textbooks — justification padding an agent consuming mid-task does not need"
status: todo
priority:
assignee:
labels: [skills, writing-quality]
depends_on: []
created: "2026-03-23T03:12:25Z"
updated: "2026-03-23T03:12:25Z"
---

## Problem

Skills should be dense operational references that an agent loads mid-task and acts on immediately. Several of the longer skills read like textbook chapters — they explain why frameworks exist, provide historical context, and justify their own methodology before getting to the actionable content. Specific examples:

1. architecture-eval/SKILL.md lines 27-29: "ATAM exists because architecture is fundamentally about tradeoffs. Optimizing one quality attribute degrades another. The goal is to make those tradeoffs explicit and deliberate rather than accidental." — three sentences of justification before the first actionable step.

2. code-review-eval/SKILL.md lines 23-26: "Google's engineering practices define an explicit priority ordering for review comments. Higher-priority issues block approval; lower-priority issues are suggestions." — two sentences explaining what Google does rather than telling the agent what to do.

3. testing-strategy/SKILL.md line 30: "Mike Cohn's original model from *Succeeding with Agile*." — bibliographic attribution an agent will never use. The entire Pyramid section spends 15 lines on explanation before the actionable "Recommended ratio" on line 46.

## Open Questions

- What's the right ratio of context to instruction in a skill? Zero context, or a single sentence max?
- Should historical attributions (Cohn, Dodds, Fowler) be kept for traceability or removed as noise?
- Are there skills that get the density right and could serve as a template?

## Why It Matters

Every line of padding in a skill is context window budget spent on text the agent won't act on. A 765-line skill that could be 400 lines is wasting roughly half the tokens it consumes on justification rather than procedure.
