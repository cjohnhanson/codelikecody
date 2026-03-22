---
title: "clc skills list and clc skills show CLI commands"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-22T03:35:39Z
updated: "2026-03-22T03:36:28Z"
---

## What

CLI commands for listing and retrieving skill content from all configured sources.

- `clc skills list` — print all available skills (name + description) from every
  configured source (local paths, git repos, built-in)
- `clc skills show <name>` — print the full SKILL.md content for a named skill.
  For built-in skills this is the only retrieval path (content lives in the binary).
  For file-based skills the agent can also read directly, but `show` works uniformly.

## Why

v7g1 added skill index injection into prime text — the agent sees skill names and
descriptions at SessionStart. But there's no CLI surface for retrieving full content.
Built-in skills especially need `show` since their content is compiled into the binary
and not on disk.

## Built-in skills

This is also where built-in skill content gets authored. Candidates:

- Missouri test authoring guide (complex scenarios, comparators, state graphs)
- Tisket workflow patterns (bulk operations, lifecycle management)
- clc workflow recipes (pickup, dispatch, coordination patterns)

Content compiled via `include_str!()` from a `skills/` directory in the clc crate.

## Depends on

- v7g1 (merged) — skill indexing and config infrastructure

## Scratch Notes
