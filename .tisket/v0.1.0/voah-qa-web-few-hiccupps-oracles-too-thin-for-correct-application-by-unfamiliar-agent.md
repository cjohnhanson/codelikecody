---
title: "qa-web FEW HICCUPPS oracles too thin for correct application by unfamiliar agent"
status: todo
priority:
assignee:
labels: [skills, quality]
depends_on: []
created: "2026-03-23T03:12:25Z"
updated: "2026-03-23T03:12:25Z"
---

## Problem

1. The FEW HICCUPPS consistency oracles in Phase 3 should be explained well enough that an agent with no prior exposure to the framework can apply each oracle correctly and distinguish a real finding from a false positive.
2. In `skills/qa-web/SKILL.md`, Phase 3 lists 11 oracles as a bullet list with one-line definitions each (e.g., "Familiar: does it work like things that work?" / "Claims: does it match what the docs say?"). There are no examples of what a violation looks like for each oracle, no guidance on how to gather evidence for or against, and no discussion of which oracles are most productive for web UI testing specifically. The paragraph after the list ("A check can pass while the test fails — the oracles catch the difference") explains the concept but doesn't operationalize any individual oracle.
3. An agent evaluating sub-agent results against these oracles has to guess what constitutes a "Familiar" violation vs. a "Comparable products" violation vs. a "Product" (internal consistency) violation. The oracles that matter most for web QA — Image, Product, Standards, Claims — aren't distinguished from the ones that rarely surface findings in this context (History, World).

## Open Questions

- Should each oracle get a concrete web-UI example (e.g., "Familiar: a save button that doesn't respond to Cmd+S violates the Familiar oracle because users expect keyboard shortcuts for save")?
- Should the oracles be prioritized for web UI context, with guidance on which ones to spend the most time on?
- Should there be a worked example showing how a single sub-agent result gets evaluated against multiple oracles?

## Why It Matters

The oracles are the decision-making core of Phase 3 — they're what turns raw pass/fail data into meaningful findings. If they're too thin to apply correctly, Phase 3 produces either false positives (everything is a violation) or misses real issues (nothing matches the vague definitions).
