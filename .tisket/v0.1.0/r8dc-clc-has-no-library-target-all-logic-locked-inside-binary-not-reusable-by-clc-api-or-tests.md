---
title: "clc has no library target — all logic locked inside binary, not reusable by clc-api or tests"
status: todo
priority:
assignee:
labels: [clc, architecture]
depends_on: []
created: "2026-03-23T03:11:52Z"
updated: "2026-03-23T03:11:52Z"
---

## Problem

The `clc` crate should expose a library target so other workspace crates (clc-api, tests, future tooling) can reuse its logic. Instead, `clc/Cargo.toml` has no `[lib]` section — it builds only as a binary. No other crate in the workspace depends on `clc` as a library (verified via Cargo.toml grep). This means all the domain logic in `clc/src/` (dispatch, gix_ops, permissions, workspace, worker management) is locked inside the binary and cannot be imported, forcing clc-api and clc-web to reimplement or shell out for functionality that already exists.

## Recent Changes (io9i)

The io9i landing moved coordination logic into clc-sdk (`coordination.rs`, `coordination_db.rs`), partially addressing this concern — the CoordinationBackend trait and its implementations are now importable by other crates. However, clc still has no `[lib]` target. Dispatch, permissions, workspace management, and gix_ops remain binary-locked in `clc/src/` with no path for external consumers to import them.

## Open Questions

- How much of `clc/src/` is reusable library code vs. CLI-specific glue? Where should the boundary be drawn?
- Should the shared logic move into `clc-sdk` instead, or should `clc` simply add a `[lib]` target alongside its `[[bin]]`?
- Does anything in clc's current module structure rely on being binary-only (e.g., `main`-specific statics)?

## Why It Matters

Without a library target, every new consumer of clc's domain logic must duplicate it or call the binary as a subprocess. The duplication is already visible (clc-api reimplements issue handling rather than importing it), and it will compound as more tooling is built on top of the workspace.
