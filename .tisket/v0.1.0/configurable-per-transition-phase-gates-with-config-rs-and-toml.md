---
title: "Configurable per-transition phase gates with config-rs and TOML"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-02-28T06:27:48Z"
updated: "2026-02-28T06:27:48Z"
---

Replace the current `.clc/config.yml` (serde_yml) config system with config-rs
and TOML. Move the config file to `clc.toml` at the project root.

## Per-transition phase gates

The `required_attempts` mechanism already exists in `phase::set()` — it tracks
an attempts counter in `.clc/state` and rejects forward transitions until the
counter reaches the threshold. Currently hardcoded to 1 everywhere.

Make it configurable per-transition in `clc.toml`:

```toml
[phases]
main_branch = "main"

[phases.gates]
tests-written = 3      # tests-unwritten → tests-written: force reconsideration
red = 1                 # tests-written → red: factual, just run tests
implementing = 1        # red → implementing: no judgment call
green = 3               # implementing → green: force reconsideration
done = 1                # green → done: clc done has own checks
```

The gate value is the number of attempts required. At attempt < threshold, the
transition is rejected with "attempt N/M: reconsider before trying again". This
forces the agent to pause and reconsider whether it's actually ready to advance.

## Defaults

If no `clc.toml` exists or `[phases.gates]` is absent, use these defaults:
- tests-written: 3
- red: 1
- implementing: 1
- green: 3
- done: 1

## Implementation

1. Add `config-rs` crate dependency (with TOML feature)
2. Rewrite `clc/src/config.rs` to use config-rs builder pattern
3. Move config file from `.clc/config.yml` to `clc.toml` at project root
4. Add per-transition gate fields to config struct
5. Wire gate values into `phase::set()` calls (currently in `clc status set`
   command handler and `maybe_bootstrap_phase`)
6. Remove `serde_yml` dependency from clc if no longer used elsewhere

## Existing code

- `clc/src/config.rs` — current config with `required_attempts: u32` (global)
- `clc/src/phase.rs` — `set()` already takes `required_attempts` param, attempts
  counter already tracked in `.clc/state`
- `clc/src/hook.rs` line 316 — `phase::set(cwd, "tests-unwritten", 1)` bootstrap
- `clc/src/main.rs` or wherever `clc status set` is handled
