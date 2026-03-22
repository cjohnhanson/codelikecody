---
title: "clc skills: aggregate and inject skill index from pluggable sources"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-22T01:26:24Z
updated: "2026-03-22T01:31:36Z"
---

## What

clc becomes a skill aggregator. It reads SKILL.md frontmatter (name + description)
from multiple configured sources and injects the index into SessionStart prime text.
The agent sees what's available and reads the full skill content on demand.

## Skill sources

Three kinds, configured in `clc.yml`:

- **Built-in** — skills compiled into the clc binary via `include_str!()`. Missouri
  authoring guides, tisket workflow patterns, clc recipes. Retrieved via
  `clc skills show <name>`.
- **Local paths** — directories on disk (e.g. `~/Projects/co.d/skills/`). Agent
  reads files directly.
- **Git repos** — remote repos cloned/cached locally (e.g.
  `git@github.com:cjohnhanson/skills.git`). Agent reads from the local clone.

## How it works

1. `clc.yml` declares skill sources under a `skills:` key
2. On SessionStart, clc scans all sources, reads SKILL.md frontmatter
3. Assembles a flat index (name + description per skill) and appends to prime text
4. Agent decides when to load a skill based on the description
5. Agent reads full SKILL.md from disk (local/git sources) or runs
   `clc skills show <name>` (built-in)

## What this is NOT

- Not a package manager (no add/remove/update/sync CLI)
- Not a replacement for `.claude/skills/` — clc is its own skill surface
- Not dynamic injection — content doesn't change based on phase/state
- Not moving prime text content into skills — prime stays deterministic enforcement

## Follows the agentskills.io standard

Skills use the standard SKILL.md format with YAML frontmatter. Same files work in
Claude Code, Cursor, Copilot, or any agent that follows the standard. clc just
aggregates and injects the index.

## Subsumes / relates to

- `bundled-docs-diataxis` — bundled docs become built-in skills instead of a
  separate `clc docs` system
- `8rb6-clc-docs-command` — `clc skills show` replaces `clc docs` for agent-facing
  content
- `contextual-skill-management` — this is the concrete implementation of the skill
  half of that design space
- `restructure-prime-text-as-skills` — separate concern (splitting prime into
  ambient/deferred); could use the skill index as the deferred surface

## Scratch Notes
