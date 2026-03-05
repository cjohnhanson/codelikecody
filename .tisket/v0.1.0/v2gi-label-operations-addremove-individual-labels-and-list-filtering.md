---
title: "Label operations: add/remove individual labels and list filtering"
status: in_progress
priority:
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: 2026-03-05T04:31:01Z
updated: "2026-03-05T04:33:04Z"
---

## Problem

Label operations are all-or-nothing. `tisket issue edit -l foo` replaces the entire label set — to add a label you need to read current labels, append, then edit. This is especially painful on trunk where file reads require `cat` workarounds.

Listing by label requires grepping output.

## Items

### `--add-label` / `--remove-label` on edit
- `tisket issue edit <id> --add-label foo` — adds `foo` to existing labels
- `tisket issue edit <id> --remove-label foo` — removes `foo`, keeps others
- No-op if label already present / already absent

### `--label` filter on list
- `tisket issue list --label foo` — only show issues with label `foo`
- The coordinator already filters by label internally but the CLI doesn't expose it

## Verification

- Issue with `labels: [a, b]` + `--add-label c` → `labels: [a, b, c]`
- Issue with `labels: [a, b, c]` + `--remove-label b` → `labels: [a, c]`
- `--add-label` on issue that already has it: no duplicate
- `--remove-label` on issue that doesn't have it: no error
- `tisket issue list --label foo` shows only issues with that label

## Scratch Notes
