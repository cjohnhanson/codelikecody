---
title: "design-review skill name-drops Nielsen and Gestalt without teaching them — weakest skill in library"
status: done
priority:
assignee:
labels: [skills, quality]
depends_on: []
created: 2026-03-23T03:12:25Z
updated: "2026-03-24T03:32:00Z"
---

## Problem

1. A skill that references evaluation frameworks should teach those frameworks well enough that an agent unfamiliar with them can apply them correctly — the skill is the agent's only source of truth during execution.
2. The `design-review` skill (`skills/design-review/SKILL.md`) references Nielsen's 10 heuristics and Gestalt principles but provides only one-line summaries for each. Phase 2 lists Nielsen's heuristics as 10 bullet points averaging ~12 words each (e.g., "Visibility of system status — is the user informed of what's happening?"). Phase 3 lists Gestalt principles as 5 bullet points of similar brevity. Compare this to `code-review-eval`, which devotes multiple paragraphs with violation examples, concrete tests, and fix guidance to each of its frameworks — or `writing-docs-eval`, which gives each DQTI characteristic a full section with check questions and violation examples.
3. An agent dispatched with this skill gets framework names and terse definitions but no examples of what violations look like, no guidance on how to distinguish severity levels within a heuristic, and no concrete test for whether a heuristic is met. The result is shallow, generic findings ("this violates consistency") rather than specific, actionable ones.

## Open Questions

- Should each of the 10 Nielsen heuristics get the same treatment as the DQTI characteristics in `writing-docs-eval` — a definition, check questions, and violation examples?
- Should the Gestalt section include visual examples or at least describe what a violation looks like in a web UI context?
- Should the Norman interaction principles (affordances, signifiers, mapping, feedback, constraints) get the same expansion, or are they secondary?

## Why It Matters

The skill is the weakest in the library by depth-of-teaching. Agents using it produce surface-level reviews because the skill doesn't give them enough to go deeper. The frameworks named are genuinely useful — the problem is that naming them isn't the same as teaching them.
