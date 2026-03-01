---
title: "Permission request system for autonomous workers"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-03-01T17:08:24Z"
updated: "2026-03-01T17:08:24Z"
---

Workers currently run with `--dangerously-skip-permissions`, which is the nuclear option — everything is allowed, no questions asked. This is bad. But the alternative (no flag) means workers block on permission prompts they can't answer because stdin is a pipe, not a terminal.

Neither extreme is right. Workers need a way to request permissions that gets routed to something that can approve or deny — the user, the coordinator, or a policy system.

This is the broader permissions model for clc-managed autonomous agents.
