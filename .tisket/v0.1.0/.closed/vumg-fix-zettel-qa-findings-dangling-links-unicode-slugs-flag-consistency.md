---
title: "Fix zettel QA findings: dangling links, unicode slugs, flag consistency"
status: done
priority:
assignee:
labels: [zettel, qa]
depends_on: []
created: 2026-03-23T01:09:46Z
updated: "2026-03-23T01:17:23Z"
---

QA run found 9 issues. Fix all of them.

## Degraded
1. Dangling links silently accepted — add zettel check command to find broken links
2. Unicode chars kept in slugs — slugify should strip diacritics, normalize to ASCII
3. read command name misleading — consider renaming or improving help text

## Cosmetic
4. -t is --tags on create/edit but --tag on list/read — normalize to --tag everywhere
5. -b short flag missing on edit --body — add it
6. orphans and stats lack --format json — add it
7. --status default on create not in bracket notation — use clap default_value

## Missing
8. Dangling link check command (zettel check)
9. Search result snippets — show matching text, not just field name
