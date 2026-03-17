---
title: "clc.toml: migrate config to TOML with config-rs and add worker permission defaults"
status: in_progress
priority: 2
assignee:
labels: [clc]
depends_on: []
created: 2026-03-17T22:00:47Z
updated: "2026-03-17T22:06:54Z"
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
