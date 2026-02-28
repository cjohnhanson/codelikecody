---
title: "Nix flake for codelikecody with home-manager integration"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-02-28T20:46:15Z"
updated: "2026-02-28T20:46:15Z"
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
