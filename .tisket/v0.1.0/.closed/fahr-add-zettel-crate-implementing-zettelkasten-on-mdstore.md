---
title: "Add zettel crate implementing zettelkasten on mdstore"
status: done
priority:
assignee:
labels: [zettel]
depends_on: [5d2z]
created: 2026-03-21T19:31:32Z
updated: "2026-03-22T01:20:07Z"
---

New crate that implements zettelkasten-style note management on top of the mdstore generic frontmattered markdown library.

## Core domain

- NoteFrontmatter: title, tags, links (forward refs by ID), created, updated
- No status workflow — notes don't have lifecycles
- Backlink computation — given a note, find everything linking to it
- Inline link syntax — [[id]] references in body text, parsed and resolved
- Link graph traversal, orphan detection

## Depends on

- mdstore for: frontmatter parse/serialize, prefix+slug IDs, directory scanning, git context
