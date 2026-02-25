---
title: "Docs site with Yew and Caddy"
status: discovery
priority:
assignee:
labels: [documentation, infrastructure]
depends_on: []
created: "2026-02-25T00:00:00Z"
updated: "2026-02-25T00:00:00Z"
---

Single Yew (Rust WASM) docs site as a new workspace member. Caddyfile serves
three subdomains off a shared base:

- `missouri.<base>.<tld>`
- `tisket.<base>.<tld>`
- `codelikecody.<base>.<tld>`

All three route to the same Yew SPA, which switches content based on subdomain.

## Open questions

- Base URL / TLD — configurable or hardcoded for now?
- One Yew binary with subdomain-based routing, or three separate builds?
- Hosting target — static files on a VPS, or WASM served from CDN?
- Content source — markdown in-repo rendered at build time, or runtime fetch?
- Relationship to bundled-docs-diataxis tisket (clc's built-in docs)
