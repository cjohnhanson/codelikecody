---
title: "missouri test cleanup kills unrelated Docker containers"
status: in_progress
priority:
assignee:
labels: [missouri, docker]
depends_on: []
created: 2026-03-26T04:04:29Z
updated: "2026-03-26T04:05:09Z"
---

## Problem

Missouri test cleanup sends SIGKILL to Docker containers it doesn't own.
When `missouri run` executes, the teardown phase kills all running
containers — not just the `clc-worker:latest` containers that missouri
spun up.

Observed killing:
- minikube containers (3 separate occasions, container vanished within
  seconds of creation)
- Coder workspace containers (`coder-admin-test-ws`, image
  `codercom/enterprise-base:ubuntu`)
- Any `clc-worker` containers from other sessions

Evidence from `docker events`:
```
container kill 2182251b... (coder.workspace_name=test-ws, signal=9)
container kill af4317bf... (image=clc-worker:latest, signal=9)
```

Both killed at the same timestamp, during a missouri test cycle.

## Expected behavior

Missouri should only clean up containers it created. Containers should
be labeled or tracked so cleanup can target only missouri-owned
resources.

## Done when

- Missouri cleanup only kills containers it started (by label, name
  pattern, or tracked container ID)
- Unrelated Docker containers survive a `missouri run` cycle
- A test verifies that a pre-existing container survives missouri
  cleanup

## Scratch Notes
