---
title: "missouri: fix due-date tests for short-id prefix filenames and unprefixed IDs"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-23T00:17:33Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

Tisket issue filenames now use a short-id prefix format (e.g., `ab12-fix-the-widget.md`) and IDs are resolved via `repo.resolve_id()` which handles full IDs, short prefixes, and legacy unprefixed names. Missouri test fixtures should exercise both the prefixed and unprefixed ID formats to ensure backward compatibility.

The due-date related missouri test fixtures (`has-issue-with-due-date`, `issue-due-date-edited`, `issue-with-due-date-closed`) use unprefixed filenames (`fix-the-widget.md`) and unprefixed IDs in their transition commands (e.g., `tisket issue edit fix-the-widget --due 2025-12-31`). Meanwhile, the `has-prefixed-issue` fixture already uses the prefixed format. There's no due-date test coverage for the prefixed ID format.

If due-date handling has edge cases with the short-id prefix system — for example, if `resolve_id` fails to match when a due-date field is present, or if the prefix generation interacts badly with due-date editing — those bugs would not be caught by the current fixture set.

## Open Questions

- Should existing due-date fixtures be migrated to prefixed filenames, or should new parallel fixtures with prefixed IDs be created alongside them?
- Are there other fixture categories (labels, tags, body editing) that also lack prefixed-ID coverage?
- Does `resolve_id` handle all three input forms (full prefixed ID, short prefix, unprefixed slug) correctly when the issue has a due_date field?

## Why It Matters

The short-id prefix is the current default for new issues. Test fixtures that only exercise the legacy unprefixed format may pass while real-world usage (which always produces prefixed IDs) fails silently.
