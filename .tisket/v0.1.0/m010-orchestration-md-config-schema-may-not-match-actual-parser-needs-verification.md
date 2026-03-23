---
title: "orchestration.md config schema may not match actual parser — needs verification"
status: discovery
priority:
assignee:
labels: [clc, docs, accuracy]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

The config examples in orchestration.md should accurately reflect what the parser accepts. Verification against config.rs reveals a conflation: orchestration.md documents both project config keys (worker permissions on line 279, coordinator policy on line 241) and topology file keys (workspaces, coordinators, inboxes, outboxes, admin on lines 386-421) on the same page. The project config (`clc.yml`) and the topology file (`clc.yaml`) are different files parsed by different code — `Config` in config.rs handles the former; the topology parser is elsewhere. orchestration.md doesn't clearly distinguish which file each schema belongs to, using headers like "Declarative Orchestration with clc.yaml" but no explicit callout that these keys go in a different file than the `worker:` and `coordinator:` keys above.

The project config keys documented in orchestration.md (worker.permissions, coordinator.auto_grant/always_escalate) do match config.rs. The topology keys have not been verified against their parser.

## Open Questions

- Where is the topology parser for `clc.yaml`? Does it match the schema shown in orchestration.md?
- The `permissions.allow` field exists in config.rs but isn't documented anywhere — is it used?
- Should orchestration.md explicitly label which keys belong in `clc.yml` vs `clc.yaml`?

## Why It Matters

A user reading orchestration.md may put topology keys (`workspaces`, `coordinators`) into `clc.yml` or project config keys (`worker.permissions`) into `clc.yaml`, getting silent parse failures or ignored fields.
