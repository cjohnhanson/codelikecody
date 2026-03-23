---
title: "clc skills: support references/ directory for progressive disclosure in product skills"
status: discovery
priority:
assignee:
labels: [clc, skills]
depends_on: []
created: 2026-03-23T01:17:46Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

The agentskills.io specification includes optional directory conventions (`scripts/`, `references/`, `assets/`) within skill directories for progressive disclosure — supplementary material an agent can pull in on demand rather than loading everything at startup.

Almanac's own documentation (`almanac/docs/what-is-almanac.md`) explicitly states it "does not yet support the optional directory conventions (`scripts/`, `references/`, `assets/`) from the spec." The product skills in `skills/` currently consist only of `SKILL.md` files with no subdirectories. There is no code in almanac or clc that discovers, indexes, or serves `references/` content.

Skills that need to provide supplementary material (examples, templates, lookup tables) have no mechanism for progressive disclosure — everything must be crammed into the SKILL.md itself, making skills either too large for startup injection or too thin for actual use.

## Open Questions

- Should `references/` content be exposed as a listing at startup (names only) with full content on demand, or discovered entirely on demand?
- Does almanac need to handle `references/` at the library level, or can clc implement it as a thin layer on top of existing skill loading?
- What's the interaction with the two skill directories (`skills/` for product skills vs `.agents/skills/` for dev skills) — do both get references support?

## Why It Matters

Without progressive disclosure for supplementary material, skill authors face a tradeoff between completeness and token cost. Reference-heavy skills (eval rubrics, template libraries, example collections) either bloat the startup context or omit information agents need mid-task.
