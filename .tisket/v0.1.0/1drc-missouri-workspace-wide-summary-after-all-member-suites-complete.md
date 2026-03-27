---
title: "missouri: workspace-wide summary after all member suites complete"
status: discovery
priority: 3
assignee:
labels: [missouri, reporting]
depends_on: []
created: "2026-03-27T12:10:35Z"
updated: "2026-03-27T12:10:35Z"
---

## Problem

When `missouri run` operates on a workspace with `members:` in the
project config, it runs each member suite sequentially and prints
results per member. There's no aggregate summary at the end — to know
if the whole workspace passed, you have to scan every member's summary
line.

With 6+ member suites (belmont, clc-api, clc, tisket, missouri, zettel,
almanac, moose), the output scrolls for pages. The overall pass/fail
answer is buried.

## Design

After all members complete, print a workspace-level summary:

```
── workspace summary ──
belmont     1 passed, 0 failed    937ms
clc-api     5 passed, 0 failed    1.5s
clc        38 passed, 0 failed    1m54s
tisket     27 passed, 0 failed    26.0s
missouri    2 passed, 0 failed    12ms
zettel      8 passed, 0 failed    13.3s
almanac     1 passed, 0 failed    161ms
moose       1 passed, 0 failed    11.4s

83 passed, 0 failed across 8 suites in 2m14s
```

If any suite failed, its line is highlighted and the slowest transitions
from that suite are shown inline.

## Open Questions

- Should the workspace summary include the slowest transitions across
  all suites, or just per-suite?
- Should failing suites be listed first in the summary?
- Should there be a workspace-level exit code (0 if all pass, 1 if any fail)?
  Currently each member's result is checked individually.

## Why It Matters

The first thing you want to know after `missouri run` on a workspace is:
did everything pass? That answer should be one line at the bottom, not
scattered across 8 separate summaries.
