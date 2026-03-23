---
title: "clc-web and clc-api HTTP types defined independently — priority field already type-mismatched"
status: todo
priority:
assignee:
labels: [clc-web, clc-api, architecture]
depends_on: []
created: "2026-03-23T03:11:40Z"
updated: "2026-03-23T03:11:40Z"
---

## Problem

The clc-web frontend and clc-api backend should share a single set of HTTP types so the API contract stays consistent. Instead, each crate defines its own types independently — `clc-web/src/types.rs` and `clc-api/src/types.rs` have diverged, with `EditIssueRequest.priority` typed as `Option<u8>` in clc-api but absent entirely from clc-web's version, and clc-api carrying fields (`assignee`, `due_date`, `append`, `depends_on`) that clc-web doesn't know about. Any field added or changed in one crate silently becomes a mismatch, producing runtime deserialization failures or dropped data that the compiler cannot catch.

## Open Questions

- Should a shared types crate live in clc-sdk, or should clc-api be the sole source of truth with clc-web importing from it?
- Are there other type divergences beyond `priority` and the missing fields that have already caused bugs in production?

## Why It Matters

Undetected type mismatches between frontend and backend produce silent data loss or runtime errors that only surface when a user hits the affected code path. The divergence will widen with every new feature.
