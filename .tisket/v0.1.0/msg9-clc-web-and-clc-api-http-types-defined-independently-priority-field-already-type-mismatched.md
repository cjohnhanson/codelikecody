---
title: "clc-web and clc-api HTTP types defined independently — priority field already type-mismatched"
status: todo
priority:
assignee:
labels: [clc-web, clc-api, architecture, standard]
depends_on: []
created: 2026-03-23T03:11:40Z
updated: "2026-04-03T18:33:27Z"
---

## Problem

The clc-web frontend and clc-api backend should share a single set of HTTP types so the API contract stays consistent. Instead, each crate defines its own types independently — `clc-web/src/types.rs` and `clc-api/src/types.rs` have diverged, with `EditIssueRequest.priority` typed as `Option<u8>` in clc-api but absent entirely from clc-web's version, and clc-api carrying fields (`assignee`, `due_date`, `append`, `depends_on`) that clc-web doesn't know about. Any field added or changed in one crate silently becomes a mismatch, producing runtime deserialization failures or dropped data that the compiler cannot catch.

## Open Questions

- Should a shared types crate live in clc-sdk, or should clc-api be the sole source of truth with clc-web importing from it?
- Are there other type divergences beyond `priority` and the missing fields that have already caused bugs in production?

## Why It Matters

Undetected type mismatches between frontend and backend produce silent data loss or runtime errors that only surface when a user hits the affected code path. The divergence will widen with every new feature.

## Scratch Notes

### Divergences found
**EditIssueRequest:**
- clc-api has `priority: Option<u8>`, clc-web has no priority field (and uses `Option<String>` elsewhere)
- clc-api has `assignee`, `due_date`, `labels`, `depends_on`, `append` — clc-web has none of these
**CreateIssueRequest:**
- clc-api has `assignee`, `due_date`, `depends_on`, `status` — clc-web missing all
- Both have `priority: Option<String>` (consistent here)

### Plan
1. Write failing tests in clc-api that demonstrate the contract mismatch (priority as String, not u8)
2. Create shared HTTP types (new crate or in clc-api with feature flag)
3. Both crates import from shared source

### Architecture decision
clc-web is WASM (leptos CSR + gloo-net), can't depend on clc-api (tokio/axum).
Need a lightweight shared types crate with only serde dependency.
Will create `clc-http-types` or add to clc-api behind a feature.
Simplest: put shared types in clc-api with no heavy deps, or new crate.

### Test approach
Write tests in clc-api showing:
1. `EditIssueRequest` priority field rejects string values (currently `u8`) — should accept strings
2. Round-trip: JSON with all fields from web client deserializes correctly in api
