---
title: "clc.yaml topology schema and config loading"
status: in_progress
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:19:43Z
updated: "2026-03-11T02:22:28Z"
---

Define the clc.yaml schema for declaring system topology: workspaces (type + agent), coordinators (workspace + selector), inboxes, outboxes, and admin config (prompt, inbox/outbox/coordinator references). Parse and validate at startup. No runtime behavior — just config loading and validation.

This is the foundational config that clc up reads to instantiate the system. Everything else references it.

Depends on: nothing
Blocks: workspace config from yaml, admin loop, clc up

## Scratch Notes
