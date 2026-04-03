---
title: "writing-docs-eval, writing-sentence-level, structured-thinking missing user-invocable frontmatter field"
status: todo
priority:
assignee:
labels: [skills, metadata, auto]
depends_on: []
created: 2026-03-23T03:12:25Z
updated: "2026-04-03T18:32:46Z"
---

## Problem

1. Every skill that can be invoked directly by a user (via `/skill-name`) should have `user-invocable: true` in its YAML frontmatter; skills that are only used as sub-components should have `user-invocable: false`. This field controls whether the skill appears in user-facing listings and tab completion.
2. Three skills are missing the `user-invocable` field entirely from their frontmatter:
   - `skills/writing-docs-eval/SKILL.md` — has `name` and `description` only
   - `skills/writing-sentence-level/SKILL.md` — has `name` and `description` only
   - `skills/structured-thinking/SKILL.md` — has `name` and `description` only
   All other skills in the library include the field (e.g., `design-review`, `qa-web`, `code-review-eval` all have `user-invocable: true`; `playwright-missouri` has `user-invocable: false`).
3. Without the field, the skill registry can't distinguish user-invocable skills from internal-only skills. Depending on how the registry handles the missing field, these skills either silently disappear from listings or appear when they shouldn't.

## Open Questions

- Should all three be `user-invocable: true`? Their descriptions suggest they're usable independently ("Use as part of writing-review or independently"), which implies yes.
- Is there validation anywhere that flags missing `user-invocable` during build or CI?

## Why It Matters

Three skills with substantial content are invisible or misclassified in the skill registry because of a missing one-line frontmatter field.
