---
title: "clc-web issue detail conflates all errors with 404"
status: discovery
priority:
assignee:
labels: [clc-web, ux]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

1. When loading an issue fails, the error should be reported accurately — a network error is not the same as a missing issue, and the user needs different information for each.
2. In `clc-web/src/pages/issue_detail.rs`, the `IssueDetail` component calls `api::get_issue(&id).await.ok()`, which converts the `Result` into an `Option` — discarding the error entirely. The `None` branch renders a hardcoded "404 / This issue doesn't exist" message regardless of what actually went wrong. Meanwhile, `api::get_issue` in `api.rs` only checks for HTTP 404 explicitly; any other non-success status (500, 403, network timeout) falls through to the JSON parse, which will produce a confusing deserialization error.
3. A server error, network failure, or permissions issue all display "This issue doesn't exist" — actively misleading the user about both the cause and the remedy.

## Open Questions

- Should `api::get_issue` check for other HTTP status codes (500, 403) and return distinct error variants?
- Should the component preserve the `Result` and render different error states for "not found" vs. "server error" vs. "network error"?
- Is the `ApiError` type rich enough to carry HTTP status codes, or does it need a richer variant structure?

## Why It Matters

Displaying "404" for a server error sends users on a wild goose chase looking for a typo in the issue ID when the real problem is infrastructure. Accurate error reporting is the minimum for a debuggable system.
