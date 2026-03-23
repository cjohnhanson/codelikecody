---
title: "Docs site: render bundled documentation as a web app"
status: discovery
priority:
assignee:
labels: [documentation, infrastructure]
depends_on: []
created: 2026-02-26T06:00:00Z
updated: "2026-03-23T02:45:00Z"
---

## Problem

Each tool has bundled docs accessible via CLI (`clc docs`, `missouri docs`,
`tisket docs`). These are markdown files baked into the binary with
`include_str!()`. The CLI interface works but isn't browsable — no
cross-linking, no search, no navigation between tools.

A previous `docs-web` crate (Leptos CSR + Trunk + Tailwind) existed and
rendered these docs as a single-page app with sidebar navigation, dark mode,
and SPA link interception. That crate was removed, but the approach was
sound.

Separately, `missouri docgen` generates documentation from test suites
(markdown and JSON formats with file contents). The `bc3w` tisket covers
rendering that output as an interactive file explorer.

## Current state

- `clc-web/` exists as a Leptos app but serves the tisket issue tracker
  UI (board view, issue detail), not documentation
- Bundled docs live in `clc/docs/`, `missouri/docs/`, `tisket/docs/`
- `missouri docgen --format json` produces structured output with file
  contents per state
- No web rendering of either bundled docs or docgen output currently exists

## Open questions

- Should docs rendering live in `clc-web` alongside the issue tracker, or
  as a separate crate?
- Should docgen output (interactive file explorer) be part of the docs site
  or a standalone tool?
- Is the previous approach (Leptos CSR + Trunk + pulldown-cmark) still the
  right one, or should bundled docs just be static HTML?
