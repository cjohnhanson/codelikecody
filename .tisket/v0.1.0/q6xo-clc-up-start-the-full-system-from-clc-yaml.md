---
title: "clc up — start the full system from clc.yaml"
status: discovery
priority:
assignee:
labels: [agents]
depends_on: []
created: "2026-03-11T02:20:27Z"
updated: "2026-03-11T02:20:27Z"
---

The top-level command that reads clc.yaml and starts the system. Instantiates workspaces, starts coordinators, begins inbox polling, runs the admin loop. Everything async, nothing blocking on human input.

This is the convergence point — all the traits (Workspace, Inbox, Outbox), the config schema, the admin loop, and the selector system come together here.

Depends on: clc.yaml schema, workspace config, inbox trait, outbox trait, admin loop, unified selectors
Blocks: nothing (this is the goal)
