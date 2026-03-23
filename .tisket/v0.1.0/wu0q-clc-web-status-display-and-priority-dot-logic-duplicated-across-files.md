---
title: "clc-web status display and priority dot logic duplicated across files"
status: todo
priority:
assignee:
labels: [clc-web, duplication]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

1. Status-to-display and priority-to-color mappings should be defined once and shared, so that adding a new status or priority level requires changing one place.
2. The mappings are duplicated across three files:
   - `board.rs` has `status_border()`, `status_accent()`, and `status_label()` — three match statements mapping status strings to CSS classes/labels.
   - `issue_detail.rs` has `status_pill()` — a fourth match statement mapping status strings to label/class pairs, using different CSS classes than `board.rs`.
   - `issue_card.rs` has `priority_dot()` — mapping priority strings to dot colors. `issue_detail.rs` has an inline match block (lines 90-95) doing the same mapping with slightly different output (`"bg-zinc-400"` vs `"bg-zinc-300 dark:bg-zinc-600"` for the fallback).
3. Adding a new status (or changing colors for an existing one) requires finding and updating four to five match statements across three files. The priority dot fallback already diverges between `issue_card.rs` and `issue_detail.rs`, which means they'll render differently for the same data.

## Open Questions

- Should these be extracted to a shared `display.rs` or `theme.rs` module?
- Should the status-to-display mapping be a single function returning a struct (label, border class, accent class, pill class), or separate functions that share a lookup table?
- Is the divergent priority fallback (`bg-zinc-400` vs `bg-zinc-300 dark:bg-zinc-600`) intentional per-context styling or an accidental drift?

## Why It Matters

Duplicated display logic is a shotgun-surgery smell — every visual change requires edits across multiple files, and the existing divergence in priority dot fallback shows the drift has already started.
