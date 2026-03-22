---
title: "docs-web: interactive file explorer for missouri doc output"
status: todo
priority: 3
assignee:
labels: [docs-web, missouri]
depends_on: [u1gm]
created: 2026-03-22T13:57:34Z
updated: "2026-03-22T13:58:03Z"
---

Render missouri doc JSON output in the docs-web Leptos app as interactive
tutorial pages.

Each state in a test path becomes a section with:
- Accordion file tree showing every file at that point (missouri-ignored
  files excluded)
- Clickable files that expand to show syntax-highlighted content
- Before/after diff view between states (what changed after the transition)
- The shell command in a console block
- The stdout/stderr output
- Embedded asciicast recordings if available (from missouri --record)

Depends on: u1gm-missouri-doc-generate-documentation-from-test-suites
