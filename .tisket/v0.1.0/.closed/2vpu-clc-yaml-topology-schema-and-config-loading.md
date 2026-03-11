---
title: "clc.yaml topology schema and config loading"
status: done
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:19:43Z
updated: "2026-03-11T02:32:34Z"
---

Define the clc.yaml schema for declaring system topology: workspaces (type + agent), coordinators (workspace + selector), inboxes, outboxes, and admin config (prompt, inbox/outbox/coordinator references). Parse and validate at startup. No runtime behavior — just config loading and validation.

This is the foundational config that clc up reads to instantiate the system. Everything else references it.

Depends on: nothing
Blocks: workspace config from yaml, admin loop, clc up

## Scratch Notes

### Plan
New module: `clc/src/topology.rs` — TopologyConfig schema + load/validate

Schema:
- `workspaces: Map<name, WorkspaceSpec { type: WorkspaceType, agent: String }>`
- `coordinators: Map<name, CoordinatorSpec { workspace: String, selector: SelectorSpec }>`
- `inboxes: Map<name, InboxSpec { path: String }>`
- `outboxes: Map<name, OutboxSpec { path: String }>`
- `admin: Option<AdminConfig { prompt: String, inbox: String, outbox: String, coordinator: String }>`

WorkspaceType enum: worker, reviewer (extensible)
SelectorSpec: label, exclude_label, project, depends_on (all optional)

Validation:
- coordinator.workspace must reference a known workspace name
- admin.inbox must reference a known inbox name
- admin.outbox must reference a known outbox name
- admin.coordinator must reference a known coordinator name

File: `clc.yaml` at project root (not in .clc/)
load() -> Result<Option<TopologyConfig>, Error>

### Files consulted
- clc/src/config.rs — existing config pattern (load from .clc/config.yml)
- clc/src/workspace.rs — workspace trait
- clc/src/coordinate.rs — coordinator patterns

### Status
Phase: tests-unwritten → need to write tests next
