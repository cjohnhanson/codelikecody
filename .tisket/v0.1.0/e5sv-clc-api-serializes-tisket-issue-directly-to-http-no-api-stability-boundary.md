---
title: "clc-api serializes tisket::Issue directly to HTTP — no API stability boundary"
status: todo
priority:
assignee:
labels: [clc-api, architecture]
depends_on: []
created: "2026-03-23T03:11:52Z"
updated: "2026-03-23T03:11:52Z"
---

## Problem

The HTTP API should have its own response types that decouple the wire format from internal data structures. Instead, `clc-api/src/handlers.rs` serializes `tisket::Issue` directly to JSON via `Json(issue)` and `Json(issues)` in `get_issue` and `list_issues`. The `tisket::Issue` struct (which derives `Serialize`) includes internal fields like `branch_statuses: Vec<BranchStatus>` and uses `serde_yml::Value` for tags — any change to the tisket crate's internal representation automatically changes the HTTP API response shape, breaking clients without any compile-time signal.

## Open Questions

- Should clc-api define its own response types and map from `tisket::Issue`, or should tisket expose a stable "public" subset?
- Are there fields on `tisket::Issue` that should never be exposed via HTTP (e.g., `branch_statuses`, internal scratch notes)?
- Does clc-web already depend on specific field names or shapes that would break if tisket's `Serialize` impl changes?

## Why It Matters

Any refactor of `tisket::Issue` — renaming a field, changing a type, adding `#[serde(skip)]` — silently changes the API contract. Clients (including clc-web) break at runtime with no compiler warning. The `search` endpoint has the same problem, returning `tisket::SearchResult` directly.
