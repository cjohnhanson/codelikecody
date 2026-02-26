---
title: "Tisket status should advance to in_progress when phase is set"
status: backlog
priority: 3
assignee:
labels: [clc, tisket]
depends_on: []
created: "2026-02-26T04:13:11Z"
updated: "2026-02-26T04:13:11Z"
---

When the phase system bootstraps (either via `clc pickup` or the SessionStart
auto-set from the sibling tisket), the matching tisket should advance from a
pickable status to `in_progress`. Currently only `clc pickup` does this
(`repo.edit_issue(id, Some("in_progress"))` on line 77 of `pickup.rs`).

Without this, a tisket can stay in `discovery` or `todo` while active work
is happening — the status becomes stale and misleading.
