---
title: "six helper functions duplicated across worker.rs, coordinator_mgmt.rs, and coordinate.rs"
status: todo
priority:
assignee:
labels: [clc, duplication]
depends_on: []
created: "2026-03-23T03:11:53Z"
updated: "2026-03-23T03:11:53Z"
---

## Problem

Process management helpers (`read_pid`, `is_process_alive`/`is_pid_alive`, `read_stdout_lines`) should be defined once and shared. Instead, they're duplicated across three files: `clc/src/worker.rs` (lines 625, 632, 637), `clc/src/coordinator_mgmt.rs` (lines 280, 287, 291), and `clc/src/coordinate.rs` (lines 503, 509). The implementations are byte-for-byte identical except for `coordinate.rs` naming its version `is_pid_alive` instead of `is_process_alive` and using a slightly different import path for `nix::sys::signal`. `read_stdout_lines` appears in both `worker.rs` and `coordinator_mgmt.rs` with the same body.

## Open Questions

- Should these live in a shared `process` or `pid` module within clc, or in clc-sdk since the workspace abstraction also does PID checking?
- `dispatch.rs` has its own `is_worker_alive` (line 143) that combines PID reading and liveness checking — should that also consolidate into the shared module?
- Is the naming inconsistency (`is_process_alive` vs `is_pid_alive`) papering over a semantic difference, or is it purely accidental?

## Recent Changes (io9i)

The duplication has grown from 3 files to 5. The io9i landing added:

- **supervisor.rs** — introduces its own `is_process_alive(u32)` variant, notably taking `u32` rather than the `i32` used elsewhere. This is a type-level inconsistency on top of the existing naming inconsistency.
- **workspace.rs** — adds `pid_alive`, yet another implementation of the same PID liveness check with yet another name.

The drift predicted in "Why It Matters" is already happening: the `u32` vs `i32` parameter type difference means these aren't even interchangeable without a cast.

## Why It Matters

A bug fix to any of these functions must be applied in three places. The naming inconsistency suggests the duplication is already causing drift. This is the kind of thing where a fix to one copy and a missed copy produces a subtle behavior difference between worker and coordinator process management.
