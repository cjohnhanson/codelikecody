---
title: "almanac: support agentskills.io directory conventions (assets/, scripts/, references/)"
status: discovery
priority:
assignee:
labels: [almanac]
depends_on: []
created: 2026-03-23T00:17:34Z
updated: "2026-03-24T03:00:00Z"
---

## Problem

The agentskills.io spec defines optional directory conventions alongside
SKILL.md: `references/` for supplementary documentation, `assets/` for
images and other binary files, and `scripts/` for executable tooling.
Almanac currently reads only `SKILL.md` from each skill directory —
everything else is ignored.

Skills that need progressive disclosure (a concise SKILL.md with detail
in `references/*.md`) or executable tooling (`scripts/`) can't use the
standard conventions. The built-in skills are all single-file because
almanac can't serve reference files.

## Open Questions

- How should `references/` work for built-in skills? `include_str!()` for
  each reference file, or a directory listing mechanism?
- Should `almanac show <skill>/references/<name>` be the retrieval path?
- What about `scripts/` — should almanac expose them, or is that out of
  scope (agents can run scripts directly from the skill directory)?
- `assets/` is primarily for rendering contexts (web, markdown). Is there
  a use case for CLI-based almanac to serve assets?

## Why It Matters

Without reference file support, skills that need depth beyond what fits
in a single SKILL.md have to cram everything into one file. The design-review
and qa-web skills were flagged as too thin — reference files would let them
keep a concise SKILL.md while having deep framework explanations available
on demand.
