---
title: "almanac: standalone skill aggregation crate"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-22T12:52:24Z
updated: "2026-03-22T12:52:55Z"
---

## What

Extract skills logic from `clc/src/skills.rs` into a standalone `almanac` crate,
following the same pattern as tisket, missouri, and zettel — own binary, own CLI,
mounted by clc as a subcommand.

## Shape

- `almanac/` crate in workspace root
- `almanac list` — list available skills (name + description + source)
- `almanac show <name>` — print full SKILL.md content
- `almanac index` — machine-readable JSON index for clc prime injection
- Standalone binary on PATH, also mountable as `clc almanac`
- Config: reads `almanac.yml` or accepts CLI flags for skill sources
- Follows agentskills.io SKILL.md standard for skill format

## Migration

- Move indexing, scanning, frontmatter parsing, show logic from `clc/src/skills.rs`
- `SkillSource` types stay shared (possibly in clc-sdk or almanac exports them)
- `clc/src/skills.rs` becomes a thin wrapper calling almanac
- Hook integration stays in clc — calls almanac for the index, formats for prime
- Missouri tests updated to use almanac binary

## Subsumes

- cgqv (merged) — `clc skills list` and `clc skills show`
- v7g1 (merged) — skill indexing and config infrastructure

## Scratch Notes
