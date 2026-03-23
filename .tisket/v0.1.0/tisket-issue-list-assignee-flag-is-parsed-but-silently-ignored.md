---
title: "tisket issue list --assignee flag is parsed but silently ignored"
status: discovery
priority: 3
assignee:
labels: [tisket, bug]
depends_on: []
created: 2026-03-19T03:40:50Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

`tisket issue list --assignee <name>` should filter the issue list to only show issues assigned to the specified person, consistent with the existing `--status` and `--label` filters.

The `--assignee` flag is declared in `IssueListArgs` (cli.rs line 171) and parsed by clap, but it is never passed to `repo.list_issues()`. The `list_issues` function signature (`repo.rs` line 348) accepts `project`, `status_filter`, `label_filter`, `closed`, and `selectors` — there is no `assignee` parameter. In the CLI handler (cli.rs line 455), the call to `repo.list_issues` passes `a.project`, `a.status`, `a.label`, `a.closed`, and `&selectors` but `a.assignee` is simply ignored.

A user who runs `tisket issue list --assignee codyhanson` gets back the full unfiltered list, with no error or warning that filtering was not applied.

## Open Questions

- Should assignee filtering be added to `repo.list_issues()` as a dedicated parameter (like status and label), or implemented via the selector system (`--where assignee=X`)?
- Does the selector system already support `assignee` matching, making the `--assignee` flag redundant?

## Why It Matters

A CLI flag that parses without error but has no effect is a bug. Users relying on `--assignee` filtering are getting incorrect (unfiltered) results with no indication anything is wrong.
