---
title: "docs-web: Leptos CSR app for serving project documentation"
status: todo
priority: 2
assignee:
labels: [docs, clc-web]
depends_on: []
created: 2026-03-21T18:23:33Z
updated: "2026-03-21T18:23:52Z"
---

Leptos CSR app that renders project documentation from markdown baked into the binary at compile time.

Follow the patterns established in clc-web (Tailwind branch):
- Leptos 0.8 CSR with trunk build
- Tailwind v4 via Trunk [tools]
- components/ and pages/ structure
- Same typography (Fraunces, DM Sans, DM Mono) and design language

Markdown source: docs/ directory (index.md, what-is-codelikecody.md, getting-started.md, clc/cli-reference.md, tisket/cli-reference.md, missouri/cli-reference.md, missouri/getting-started.md)

Approach:
- include_str!() or build.rs to bake markdown into the binary
- pulldown-cmark or similar for markdown → HTML rendering
- Client-side syntax highlighting for code blocks
- Sidebar navigation matching the doc structure
- Search (client-side, full-text over the baked content)

Separate crate: docs-web/ in the workspace. Not part of clc-web.
