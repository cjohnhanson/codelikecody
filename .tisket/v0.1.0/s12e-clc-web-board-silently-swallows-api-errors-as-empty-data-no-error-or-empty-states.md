---
title: "clc-web board silently swallows API errors as empty data — no error or empty states"
status: discovery
priority:
assignee:
labels: [clc-web, ux]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

1. When an API call fails, the board should display an error state so the user knows something went wrong and can take action.
2. In `clc-web/src/pages/board.rs`, the `Board` component calls `api::list_issues(...).await.unwrap_or_default()`, which converts any API error — network failure, server 500, malformed JSON — into an empty `Vec`. The board then renders zero columns with no indication that anything failed.
3. A user facing a backend outage or misconfiguration sees an empty board indistinguishable from a project with no issues. There is also no empty state for the legitimate case of zero open issues — it just renders nothing below the "0 open" count.

## Open Questions

- Should the error state offer a retry action, or just display the error message?
- Should an empty-but-successful response ("0 open issues") have its own distinct empty state, separate from the error state?
- Is `LocalResource` the right primitive here, or would switching to a resource that exposes the `Result` directly simplify error handling?

## Why It Matters

Silent data loss is the worst UX failure mode — the user trusts what they see. An empty board during an outage looks like the project is empty, not that the system is broken.
