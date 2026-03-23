---
title: "code-review-eval cognitive complexity rules contradict on whether else increments the score"
status: todo
priority:
assignee:
labels: [skills, accuracy]
depends_on: []
created: "2026-03-23T03:12:25Z"
updated: "2026-03-23T03:12:25Z"
---

## Problem

1. The cognitive complexity rules should be internally consistent so an agent applying them produces a deterministic score for the same code.
2. In `skills/code-review-eval/SKILL.md`, Rule 1 (line ~430) lists `else` as a flow break that increments the score: "`if`, `else if`, `else`" all appear in the same bullet. Rule 3 (line ~453) then says: "`else` after `if` when it simplifies understanding (but `else if` does increment because it's a new condition)" does NOT add to complexity. These two rules directly contradict — Rule 1 says `else` increments, Rule 3 says it doesn't.
3. An agent following Rule 1 will score `else` branches as +1. An agent following Rule 3 will skip them. The same code gets different complexity scores depending on which rule the agent encounters last or weighs more heavily. This makes the metric unreliable and the review findings inconsistent.

## Open Questions

- Which rule reflects the intended behavior? The SonarSource specification (the original source for cognitive complexity) does NOT count bare `else` as an increment — only `else if` increments because it introduces a new condition. This aligns with Rule 3.
- Should Rule 1's bullet be corrected to "`if`, `else if`" (removing `else`)?
- Are there other inconsistencies between Rule 1 and Rule 3 that should be audited?

## Why It Matters

Cognitive complexity is used as a threshold check (the skill recommends 15 per function). A contradiction in the counting rules means the threshold is applied inconsistently — functions that should be flagged aren't, or functions that are fine get flagged, depending on which rule wins.
