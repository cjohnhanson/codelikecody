---
title: "Docs site with Leptos and Tailwind"
status: discovery
priority:
assignee:
labels: [documentation, infrastructure]
depends_on: []
created: 2026-02-26T06:00:00Z
updated: "2026-03-02T08:00:00Z"
---

Epic: docs site built with Leptos (Rust WASM), styled with Tailwind, content sourced from markdown files in this repo following diataxis structure.

## Vision

Markdown docs live in this repo (e.g., `docs/`), organized in a diataxis structure (tutorials, how-to, reference, explanation). The Leptos site builds from them — either at compile time or with a build step that converts markdown to something Leptos can render. Single site covering all three projects (clc, missouri, tisket).

## Why this matters beyond docs

The docs site is the opportunity to start dogfooding browser automation. The development workflow — missouri tests → red → green → review → ship — applies even to frontend work. Browser tests (via Playwright or similar) can assert that pages render, navigation works, content is correct. This makes the docs site the proving ground for the browser testing infrastructure.

## Epic breakdown

### 1. Scaffolding (pickable NOW)

Get to a point where `just serve-docs` runs locally and shows a landing page.

- Add `docs-site/` as a workspace member (or separate crate — Leptos has its own build story with `cargo-leptos` or Trunk)
- Leptos app with Tailwind CSS
- Simple landing page — project name, brief description, nav skeleton
- `just serve-docs` starts the dev server
- Add Leptos/Trunk/wasm tooling to `flake.nix` devShell

### 2. Markdown pipeline

- Diataxis directory structure in `docs/` (tutorials/, howto/, reference/, explanation/)
- Build step that converts markdown to rendered content in the Leptos app
- Syntax highlighting for code blocks
- Navigation generated from directory structure

### 3. Styling and design

- Tailwind configuration and theme
- Responsive layout
- Code block styling
- Search (maybe — could be a later epic)

### 4. Browser testing integration

- Missouri tests or Playwright tests that assert page content
- This is the first real use of browser automation in the project
- Tests run as part of the standard test suite

### 5. Deployment

- Static build output
- Hosting target TBD (CDN, VPS, GitHub Pages)
- Subdomain routing (missouri.*, tisket.*, codelikecody.*)

## What's pickable now

Item 1 — scaffolding. Everything else depends on it. The tisket for this specific work should be scoped separately when we're ready to dispatch it.
