---
title: "cross-reference links use doc-slugs that break outside doc-serving system"
status: todo
priority:
assignee:
labels: [docs, linking]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

Cross-references between docs should work both in a doc-serving system and when reading the raw markdown files. Currently, all cross-references use slug paths like `(/clc/cli-reference)`, `(/missouri/what-is-missouri)`, `(/tisket/workflow)`. These resolve in the docs web server but are broken links when reading the markdown files directly on disk or on GitHub. For example, what-is-codelikecody.md contains links like `[clc CLI reference](/clc/cli-reference)`, `[missouri getting started tutorial](/missouri/getting-started)`, and `[CLI reference](/missouri/cli-reference)` — none of which resolve to actual file paths.

The docs are bundled into the binary and served via `almanac docs`, so the slug format works there. But agents reading docs via `cat` or file tools — and humans browsing on GitHub — get dead links.

## Open Questions

- Is the doc-serving system the primary reading context, making relative `.md` paths the wrong default?
- Could links use a format that works in both contexts (e.g., relative paths that the server also resolves)?
- How many total cross-reference links exist across the corpus?

## Why It Matters

Agents that follow a "see also" link by reading the referenced path get a file-not-found. The docs claim to be interconnected but the connections only work through one specific rendering system.
