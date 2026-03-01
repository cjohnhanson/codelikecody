---
title: "Permission request system for autonomous workers"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-01T17:08:24Z
updated: "2026-03-01T17:28:56Z"
---

Workers currently run with `--dangerously-skip-permissions`, which is the nuclear option — everything is allowed, no questions asked. This is bad. But the alternative (no flag) means workers block on permission prompts they can't answer because stdin is a pipe, not a terminal.

Neither extreme is right. Workers need a way to request permissions that gets routed to something that can approve or deny — the user, the coordinator, or a policy system.

This is the broader permissions model for clc-managed autonomous agents.

## Approach

Workers get clc/tisket/missouri permissions by default — the tools they need for the workflow. Everything else requires escalation.

When a worker needs a permission it doesn't have:
1. Worker calls `clc permissions request "description of what and why"`
2. Worker stops (blocks, doesn't proceed)
3. Coordinator sees the request via monitoring (`clc worker <id> check`)
4. Coordinator evaluates whether it's reasonable and safe
5. If approved: `clc permissions grant <worker-id> <permission>`
6. Worker resumes with the new permission accumulated

Permissions accumulate over the worker's lifetime — once granted, they persist for that worker session. This means early requests establish the permission set and later work flows without interruption.

The coordinator becomes the permission authority, not `--dangerously-skip-permissions`.
