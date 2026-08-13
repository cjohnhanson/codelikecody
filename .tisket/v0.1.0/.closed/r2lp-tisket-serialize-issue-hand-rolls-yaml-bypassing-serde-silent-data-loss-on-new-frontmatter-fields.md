---
title: "tisket::serialize_issue hand-rolls YAML bypassing serde — silent data loss on new frontmatter fields"
status: done
priority:
assignee:
labels: [tisket, correctness, blocking, standard]
depends_on: []
created: 2026-03-23T03:11:53Z
updated: "2026-08-13T17:47:00Z"
---

## Problem

`serialize_issue` in `tisket/src/issue.rs` (line 156) should use serde to serialize `IssueFrontmatter`, ensuring every field round-trips correctly. Instead, it hand-rolls YAML output with `format!` string concatenation — individually formatting `title`, `status`, `priority`, `assignee`, `due_date`, `labels`, `depends_on`, `created`, `updated`, and `tags`. When a new field is added to `IssueFrontmatter` (the struct derives `Deserialize`), parsing picks it up automatically, but `serialize_issue` silently drops it on write because the hand-rolled code doesn't know about the new field. The `tags` field was clearly a late addition (it's handled separately at the end), suggesting this has already happened at least once.

## Open Questions

- Is the hand-rolling intentional to preserve a specific YAML field order or formatting style that serde_yml doesn't support?
- Does the current code correctly handle special characters in field values (e.g., titles containing colons, quotes, or YAML-significant characters)?
- Would switching to serde_yml for serialization change the file format enough to break existing issue files on round-trip?

## Why It Matters

Any new frontmatter field added to `IssueFrontmatter` will parse correctly but silently vanish on the next write. The compiler provides no warning because `serialize_issue` takes `&IssueFrontmatter` — it has access to all fields, it just doesn't use them. This is a data loss bug waiting to happen on every schema change.
