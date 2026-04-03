---
title: "add moose + chromium to worker Docker image for frontend visual verification"
status: todo
priority: 2
assignee:
labels: [clc, auto]
depends_on: []
created: "2026-04-03T19:37:05Z"
updated: "2026-04-03T19:37:05Z"
---

## Problem

The worker Docker image includes `clc`, `tisket`, and `missouri` but not
`moose` (browser automation) or chromium. Frontend workflows need visual
verification — a reviewer agent should be able to screenshot a rendered
page and evaluate it. Without a browser in the container, the `design-review`
reviewer can only do code-level review, not visual QA.

## Proposed solution

Two changes to `docker/worker/Dockerfile`:

1. **Builder stage**: copy `moose` binary alongside clc/tisket/missouri
   (line 25 — add `target/debug/moose` to the cp command)

2. **Runtime stage**: install chromium and its dependencies
   ```
   apt-get install -y chromium
   ```
   Chromium in Debian bookworm pulls ~200MB of deps. The image is already
   ~1GB so this is proportional.

Also add `almanac` to the copied binaries — workers should have access to
the skill system directly, not just via `clc`.

## Done When

- `moose --version` works inside a worker container
- `moose screenshot https://example.com /tmp/test.png` produces a screenshot
- `almanac list` works inside a worker container
- `chromium --version` works inside a worker container
- Docker image builds successfully (via Depot or local)
- Existing worker functionality is unaffected (clc, tisket, missouri still work)
