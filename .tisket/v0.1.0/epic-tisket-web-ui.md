---
title: "Epic: tisket web UI"
status: discovery
priority:
assignee:
labels: [epic, web]
depends_on: []
created: "2026-03-18T02:11:21Z"
updated: "2026-03-18T02:11:21Z"
---

Web interface for tisket issue tracking, built with Leptos (CSR) and
Axum API layer. Tauri desktop client to follow later.

## Architecture

- **clc-api** — Axum API wrapping the tisket lib crate (Repo, Issue, etc.)
- **clc-web** — Leptos CSR frontend, built with Trunk
- **Missouri services** — new primitive for testing APIs (service lifecycle
  scoped to transitions and assertions)

## Dependency chain

1. Missouri services primitive (enables API testing)
2. clc-api crate (API layer, tested with missouri services)
3. Missouri API tests (e2e verification)
4. clc-web crate (frontend scaffold)
5. Tisket views (list, detail, mutations, search)

## Design decisions

- CSR, not SSR — simpler for Tauri shell later
- Port 0 for services — OS assigns, service reports port via stderr,
  missouri captures and injects $PORT into transition/assertion env
- services: key on TransitionConfig and AssertionConfig, not state-level —
  different transitions may need different services, YAML anchors for reuse
- Filesystem comparison happens with services stopped — files on disk are
  the source of truth, no races with running processes
