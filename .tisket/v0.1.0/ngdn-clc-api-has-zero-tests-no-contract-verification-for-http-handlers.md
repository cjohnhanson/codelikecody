---
title: "clc-api has zero tests — no contract verification for HTTP handlers"
status: discovery
priority:
assignee:
labels: [clc-api, testing]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

1. The `clc-api` crate (416 lines across 5 source files) exposes an Axum HTTP API with 9 handler endpoints — `health`, `list_projects`, `list_issues`, `get_issue`, `create_issue`, `edit_issue`, `close_issue`, `reopen_issue`, and `search`. Each handler's request/response contract should be verified by tests, especially since a separate tisket (`e5sv`) documents that the API serializes `tisket::Issue` directly to HTTP responses with no stability boundary.
2. There are zero `#[test]` attributes and no `#[cfg(test)]` modules anywhere in `clc-api/src/`. The handlers in `handlers.rs` (145 lines) convert between HTTP types and tisket repo operations with error mapping — none of which is verified. The `types.rs` (61 lines) defines request/response types that are already known to have a field mismatch with `clc-web` (tisket `msg9`).
3. API contract changes — field renames, type changes, new required fields — ship with no automated verification. The frontend (`clc-web`) and any future API consumers discover breakage at runtime. The known priority field type mismatch between API and web (tisket `msg9`) is exactly the kind of bug tests would catch.

## Open Questions

- Should tests use Axum's test utilities (`TestClient` / `oneshot`) against an in-memory tisket repo, or should they run the full server against a temp directory?
- Is the lack of an API stability boundary (tisket `e5sv`) a prerequisite, or can contract tests be written against the current shape and updated when a proper response type is introduced?
- What's the minimum coverage: just the happy paths, or also error cases (missing project, invalid ID, malformed request body)?

## Why It Matters

The API is the bridge between the web UI and the tisket data layer. Without contract tests, any change to tisket's data model or the API's serialization silently breaks the frontend. The type mismatch bug already exists — tests would have prevented it and will prevent the next one.
