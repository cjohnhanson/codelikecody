---
title: "clc-api crate — Axum API wrapping tisket lib"
status: todo
priority:
assignee:
labels: [web, feature]
depends_on: [missouri-services-primitive-for-transitions-and-assertions]
created: 2026-03-18T02:11:51Z
updated: "2026-03-18T02:34:47Z"
---

New workspace member `clc-api`. Axum HTTP API wrapping the tisket lib crate.

## Endpoints

- `GET /health` — readiness check (required for missouri services)
- `GET /api/projects` — list projects
- `GET /api/issues?project=&status=` — list issues (filters optional)
- `GET /api/issues/:id` — single issue detail
- `POST /api/issues` — create issue
- `PATCH /api/issues/:id` — edit issue
- `POST /api/issues/:id/close` — close issue
- `POST /api/issues/:id/reopen` — reopen issue
- `GET /api/search?q=&project=` — full-text search

## Notes

- `IssueFrontmatter` needs `Serialize` derive (currently only `Deserialize`)
- `Issue` needs `Serialize` derive
- Repo initialized from a root path passed via CLI flag or env var
- Port 0 support: bind to 0, print actual port to stderr for missouri
- JSON responses throughout, serde for request/response types
