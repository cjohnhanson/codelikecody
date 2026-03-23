---
title: "Investigate missouri test suite performance"
status: done
priority:
assignee:
labels: [missouri]
depends_on: []
created: 2026-03-19T02:36:01Z
updated: "2026-03-23T02:12:21Z"
---

## Problem

Observed missouri test runs taking 5+ minutes during worker execution (`missouri run -d clc/tests/missouri`). Workers were running two parallel missouri invocations which may have compounded the issue, but even a single run shouldn't take that long for filesystem state graph tests.

## Investigation areas

- Profile individual test transitions to find the slow ones
- Check if network interception tests (mitmproxy) are disproportionately slow
- Check if subprocess spawning per transition is the bottleneck
- Check if parallel missouri runs on the same machine cause resource contention
- Consider whether missouri needs a timing/profiling report mode

## Scratch Notes

### 2026-03-19 — first profiling run

Full run: `missouri run -d clc/tests/missouri -v` — **135m48s** total
- 31 paths, 120 transitions, 3831 assertions
- 29 passed, 2 failed (pre-existing failures, investigating separately)
- Setup step (`cargo build`) completes quickly — not the bottleneck

Key observations:
- ~28 of 31 paths share `bare-project → initialized` prefix, re-executed every time
- `initialized` state has massive assertion/transition surface (40KB+ missouri.yml)
- Individual path times: 16s (trivial) to 5m28s (has-tisket)
- `workers-merged → integration-landed` transition alone: 1m23s
- 1% CPU utilization — likely I/O bound or waiting on subprocess spawning

Second run in progress with latest main (includes test fixes). Comparing.
