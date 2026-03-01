---
title: "Document missouri comparators so they're usable without reading source"
status: in_progress
priority: 3
assignee:
labels: [docs, missouri]
depends_on: []
created: 2026-02-26T03:57:40Z
updated: "2026-03-01T16:51:47Z"
---

Missouri's custom comparator system (per-file comparison scripts in `.missouri/bin/`)
is undocumented. The only way to learn how comparators work is to read missouri's
source code or reverse-engineer the existing `04-custom-comparator` test fixture.

Most projects using missouri won't have access to the source. Comparators need
documentation that covers:

- What a comparator script receives (`$1` = expected file, `$2` = actual file)
- How to register a comparator in `missouri.yml` (`comparators.files[].command`)
- Where comparator scripts live (`.missouri/bin/`)
- Common patterns: line-count comparison, JSON semantic comparison, regex matching
- How comparators interact with the `ignore` flag
- Environment variables available to comparator scripts
