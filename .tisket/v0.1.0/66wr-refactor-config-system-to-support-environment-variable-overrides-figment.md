---
title: "Refactor config system to support environment variable overrides (figment)"
status: discovery
priority: 3
assignee:
labels: [clc, config]
depends_on: []
created: "2026-03-24T01:20:53Z"
updated: "2026-03-24T01:20:53Z"
---

Replace the hand-rolled config loading (YAML → TOML → defaults) with
figment, which merges config sources with a priority order:

1. Defaults (code)
2. Config file (clc.yml)
3. Environment variables (CLC_ prefix)

Every config value becomes overridable via env var. Examples:
- `CLC_MAIN_BRANCH=trunk`
- `CLC_SUPERVISOR_POLL_INTERVAL=5`
- `CLC_SUPERVISOR_API_PORT=19876`

Current config system is in `clc/src/config.rs` — serde deserialization
from YAML/TOML with manual defaults. figment replaces this with a
single `Figment::from(Serialized::defaults(Config::default()))
.merge(Yaml::file("clc.yml")).merge(Env::prefixed("CLC_"))` chain.
