---
title: "clc-web api::search query params not URL-encoded — broken requests with special characters"
status: todo
priority:
assignee:
labels: [clc-web, correctness, blocking, auto]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-04-03T18:32:47Z"
---

## Problem

1. Query parameters interpolated into URLs should be percent-encoded so that special characters (`&`, `=`, `+`, spaces, unicode) are transmitted correctly.
2. In `clc-web/src/api.rs`, both `search()` and `list_issues()` interpolate values directly into the URL string with `format!()` — no encoding is applied. `search()` does `format!("/api/search?q={q}")` and `list_issues()` does `format!("project={p}&")` / `format!("status={s}&")`.
3. Any query containing `&`, `=`, `#`, spaces, or non-ASCII characters will produce a malformed URL — the server receives truncated or garbled parameters, returning wrong results or errors with no indication to the user why.

## Open Questions

- Does `gloo_net::http::Request` provide a URL builder or query-parameter helper that handles encoding, or does a separate crate (`web_sys::UrlSearchParams`, `js_sys`) need to be used?
- Are there other API functions that interpolate path segments (e.g., `get_issue`, `edit_issue`) where the `id` parameter could contain problematic characters?

## Why It Matters

Search is a primary discovery mechanism. Queries with spaces or punctuation — the common case for natural-language search — silently return wrong results or fail entirely.
