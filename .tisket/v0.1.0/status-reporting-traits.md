---
title: "Status reporting traits"
status: in_progress
priority:
assignee:
labels: [architecture]
depends_on: [clc-sdk-crate-with-agent-detection]
created: 2026-02-24T14:52:06Z
updated: "2026-02-25T01:51:49Z"
---

Implement `ClcTool::status_basic()` and `ClcTool::status_full()` across the
ecosystem.

## status_basic

One-liner summary for periodic context reinforcement. Examples:

- tisket: `"tisket: test-feature (in_progress), 3 open in v0.1.0"`
- missouri: `"missouri: 10/10 passing (433 assertions)"`

## status_full

Complete state dump for SessionStart on feature branches. Examples:

- tisket: full issue body including scratch section, all open issues by status
- missouri: test results by state, last run timestamp, any failures

## Implementation

1. tisket implements both methods using its `Repo` API
2. missouri implements both methods using its run/report data
3. clc aggregates status from all mounted tools
4. `clc status` already exists — extend it to call these trait methods
5. Hook handlers use status_basic for reinforcement, status_full at SessionStart
