---
title: "Nix flake for codelikecody with home-manager integration"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T20:46:15Z
updated: "2026-03-01T14:51:00Z"
---

## Problem

clc is currently only available via `cargo install` or by having `target/debug` on PATH. Missouri tests fail with `clc: command not found` when run in clean shell contexts because there's no proper installation path.

## What

1. Add `flake.nix` to this repo that builds the clc binary (and any other workspace binaries) as a Nix package
2. In `co.d/home.nix`, reference the codelikecody flake from GitHub to install clc system-wide
3. Push main to GitHub continuously so the flake stays current

## Shape

- `flake.nix` at repo root using crane or naersk for Rust builds
- Workspace has multiple crates (clc, tisket, missouri, clc-sdk) — flake should build clc as the primary package, possibly expose others
- home.nix adds the flake input and includes clc in `home.packages`
- missouri tests should just work once clc is on the nix-managed PATH

## Scratch Notes

### What was done
1. **flake.nix** — crane-based Rust build, stable toolchain, cleanSourceWith for test fixtures, nativeCheckInputs for test deps, MISSOURI_SANDBOX=preinstalled for in-nix test execution
2. **Missouri flox→nix migration** — replaced entire flox sandbox backend with `nix shell`, added `Sandbox::None`/`Sandbox::Nix` enum, `detect_sandbox()` with preinstalled env var support, updated executor/compare/recorder/config/graph/error
3. **Test fixtures** — 08-dbt and 11-cargo converted from flox manifests to packages lists
4. **Illinois meta-tests** — all flox references → nix, env passthrough for HOME/TMPDIR
5. **GitHub repo** — `cjohnhanson/codelikecody` (private), main pushed
6. **co.d integration** — `co.d/flake.nix` updated with codelikecody input (git+ssh for private), package added to sharedModules home.packages
7. **Three nix build iterations** — first had 2 test failures (preinstalled env leak + no nix on PATH), second had ordering bug, third passed clean

### Commits on branch
- `387cadc` feat(missouri): replace flox sandbox backend with nix shell
- `3a2832a` fix(flake): use MISSOURI_SANDBOX=preinstalled, drop flox input
- `56bb7f2` fix: check nix availability before calling detect_sandbox in test

### Remaining for user
- Commit co.d changes and run `hms` to deploy clc system-wide
- Push main to GitHub after each `clc done` merge going forward
