---
title: "clc.yml config schema reference missing — keys documented only in orchestration guide"
status: todo
priority:
assignee:
labels: [clc, docs, completeness]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

The clc cli-reference.md `clc config` section should be the authoritative schema reference for `clc.yml`. Instead, it only describes the config file precedence order (clc.yml > clc.toml > .clc/config.yml) and the `clc config show` command. The actual config keys — `main_branch`, `admin_branch`, `required_attempts`, `permissions.allow`, `worker.permissions.default`, `worker.permissions.deny`, `coordinator.auto_grant`, `coordinator.always_escalate`, `workflows`, `rules`, `skills` — are nowhere in cli-reference.md. Some appear as examples in orchestration.md (worker permissions on line 279, coordinator policy on line 241), but there's no single place that lists every key, its type, its default, and what it does.

The parser (config.rs) accepts 11 top-level fields across nested structs. The cli-reference documents zero of them.

## Open Questions

- Should the schema reference live in cli-reference.md under `clc config`, or in a dedicated config-reference page?
- The `permissions.allow` field exists in the Config struct but doesn't appear in any doc — what does it do?
- Are the `workflows` and `rules` fields stable enough to document, or still experimental?

## Why It Matters

Users configuring `clc.yml` have to reverse-engineer the schema from orchestration.md examples and source code. Agents generating config files have no reference to validate against.
