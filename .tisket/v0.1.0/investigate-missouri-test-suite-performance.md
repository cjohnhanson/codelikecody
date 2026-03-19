---
title: "Investigate missouri test suite performance"
status: in_progress
priority:
assignee:
labels: [missouri]
depends_on: []
created: 2026-03-19T02:36:01Z
updated: "2026-03-19T03:41:00Z"
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
