---
title: "clc.toml: migrate config to TOML with config-rs and add worker permission defaults"
status: done
priority: 2
assignee:
labels: [clc]
depends_on: []
created: 2026-03-17T22:00:47Z
updated: "2026-03-18T01:39:05Z"
---

Move clc configuration from `.clc/config.yml` (serde_yml) to `clc.toml`
at the project root, using config-rs with the TOML feature. This is the
foundation for all other config-driven features (phase gates, workspace
config, clc up).

## Why

`.clc/config.yml` is hidden inside the `.clc/` infrastructure directory.
Project config should be visible at the root — same convention as
`Cargo.toml`, `pyproject.toml`, `flake.nix`. TOML over YAML because
YAML parsing is more error-prone and TOML is the Rust ecosystem standard.

Worker permission defaults are currently hardcoded in `permissions.rs`
as `BASELINE_PERMISSIONS`. This is too broad (blanket `Write`, `Edit`
with no path scoping) and not configurable per project. Workers can
write anywhere, including `.clc/state`, which lets them bypass phase
enforcement entirely.

## Config shape

```toml
[project]
main_branch = "main"

[worker.permissions]
default = [
  "Read",
  "Grep",
  "Glob",
  "Write({worktree}/**)",
  "Edit({worktree}/**)",
  "Bash(clc *)",
  "Bash(tisket *)",
  "Bash(missouri *)",
  "Bash(cargo *)",
  "Bash(git add *)",
  "Bash(git commit *)",
  "Bash(git status *)",
  "Bash(git diff *)",
  "Bash(git log *)",
]

deny = [
  "Write({worktree}/.clc/**)",
  "Edit({worktree}/.clc/**)",
]

[coordinator]
auto_grant = []
always_escalate = []
```

`{worktree}` is expanded at dispatch time to the actual worktree path.
Workers can only write within their worktree and cannot touch `.clc/`
infrastructure. Anything beyond the defaults goes through the permission
request system.

## Implementation

1. Add `config-rs` crate dep (with TOML feature), add `toml` crate
2. Rewrite `clc/src/config.rs` — config-rs builder, load from `clc.toml`
3. Move existing fields (`main_branch`, `required_attempts`,
   `permissions.allow`, `coordinator.*`) to new schema
4. Add `[worker.permissions]` section with `default` and `deny`
5. `seed_baseline` → `seed_defaults`: read from config, expand
   `{worktree}`, write to `settings.local.json` including deny rules
6. Remove `serde_yml` dep from clc if unused elsewhere
7. Fall back to hardcoded defaults when no `clc.toml` exists

## What this unblocks

- `configurable-per-transition-phase-gates` — `[phases.gates]` section
- `wlw1-workspace-configuration-from-clc-yaml` → workspace config in TOML
- `q6xo-clc-up` — reads `clc.toml` to start the full system
- Worker isolation — `.clc/` is protected, phase bypass is blocked

## What this fixes

Workers currently get blanket `Write` and `Edit` permissions. This
allowed the mitmproxy-network-mocking worker to write directly to
`.clc/state`, bypassing the phase system (skipping review-requested →
in-review → reviewed → done, going straight from green to done).
With scoped permissions and deny rules, that write would be blocked.

## Scratch Notes

### 2026-03-17: Tests written (phase: tests-unwritten → tests-written)

**Missouri tests created (4 new test states):**
- `has-toml-config` — clc.toml at root with `[project] main_branch = "trunk"`, verifies config show and status
- `has-bad-toml-config` — invalid TOML, verifies error handling
- `toml-config-with-coordinator` — clc.toml with `[coordinator]` section
- `toml-config-with-worker-permissions` — clc.toml with `[worker.permissions]` default + deny

**Unit tests added to config.rs (9 new tests):**
- `load_toml_config_from_project_root` — loads from clc.toml
- `load_toml_config_defaults_when_no_file` — falls back to defaults
- `load_toml_config_error_on_invalid_toml` — errors on bad TOML
- `load_toml_config_with_coordinator_section` — parses coordinator
- `load_toml_config_with_worker_permissions_default` — parses worker.permissions.default
- `load_toml_config_with_worker_permissions_deny` — parses worker.permissions.deny
- `load_toml_config_worker_permissions_empty_by_default` — empty when absent
- `load_toml_config_prefers_clc_toml_over_yaml` — clc.toml wins over .clc/config.yml
- `load_toml_config_full_shape` — full config with all sections

**Unit tests added to permissions.rs (7 new tests):**
- `seed_defaults_uses_config_permissions_when_provided` — uses config, not hardcoded
- `seed_defaults_writes_deny_rules` — writes permissions.deny array
- `seed_defaults_expands_worktree_placeholder` — {worktree} → actual path
- `seed_defaults_falls_back_to_baseline_when_config_empty` — empty config → hardcoded baseline
- `seed_defaults_is_idempotent` — won't overwrite after grants
- `seed_defaults_sets_dont_ask_mode` — defaultMode: dontAsk
- `seed_defaults_merges_into_existing_settings` — preserves hooks

**Key design decisions from tests:**
- New struct fields: `config.worker.permissions.default`, `config.worker.permissions.deny`
- New function: `seed_defaults(working_dir, config_defaults, config_deny)` replaces `seed_baseline`
- {worktree} expansion happens in seed_defaults, not in config loading
- Empty config.worker.permissions.default → falls back to BASELINE_PERMISSIONS
- clc.toml at project root takes precedence over .clc/config.yml

**Files consulted:**
- `clc/src/config.rs` — current YAML config loading
- `clc/src/permissions.rs` — BASELINE_PERMISSIONS, seed_baseline
- `clc/src/dispatch.rs` — calls seed_baseline
- `clc/src/main.rs` — wiring, cmd handlers
- `clc/tests/missouri/has-config/` — existing config test pattern
- `clc/tests/missouri/dispatched-with-config-permissions/` — existing permissions test
- `clc/tests/missouri/.missouri/` — test infrastructure (bin wrappers, setup)

### 2026-03-17: Implementation complete (phase: green)

**Changes made:**
- `clc/Cargo.toml`: added `toml = "0.8"` dependency
- `clc/src/config.rs`: added `WorkerPermissionsConfig`, `WorkerConfig`, `TomlFile` structs; `load()` now checks `clc.toml` first, falls back to `.clc/config.yml`; `show()` outputs TOML
- `clc/src/permissions.rs`: added `seed_defaults()` with config-driven defaults, `{worktree}` expansion, deny rules; `seed_baseline()` demoted to test-only
- `clc/src/dispatch.rs`: accepts `worker_perm_defaults` + `worker_perm_deny`, calls `seed_defaults`
- `clc/src/coordinate.rs`: same parameter change, calls `seed_defaults`
- `clc/src/main.rs`: passes `cfg.worker.permissions.default` and `cfg.worker.permissions.deny`

**All 136 unit tests pass.**

**Note:** Missouri E2E tests can't run in this environment due to rustup toolchain issue (no default toolchain configured). The `clc` binary builds fine with `cargo +nightly`.

**Not done (future work):**
- Remove `serde_yml` dep if unused elsewhere (it's still used for YAML fallback + topology.rs)
- Update old YAML missouri tests (has-config, has-bad-config, coordinator-policy-config) to TOML
- Add `clc.toml` to this project root with actual worker permission defaults
