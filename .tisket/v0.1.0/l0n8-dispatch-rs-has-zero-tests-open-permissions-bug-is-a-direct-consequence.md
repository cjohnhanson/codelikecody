---
title: "dispatch.rs has zero tests — open permissions bug is a direct consequence"
status: todo
priority:
assignee:
labels: [clc, testing, blocking, standard]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-04-03T18:33:27Z"
---

## Problem

1. `clc/src/dispatch.rs` (259 lines) orchestrates the full worker lifecycle: worktree creation via `pickup`, permission seeding via `permissions::seed_defaults`, FIFO pipe setup, agent process spawning, and stale worktree cleanup. Each step should be independently testable, and the permission-seeding path in particular should verify that config-specified `permissions.allow` entries actually reach the worktree's `.claude/settings.local.json`.
2. There are zero `#[test]` attributes and no `#[cfg(test)]` module in `dispatch.rs`. The open permissions propagation bug (tisket `clc-dispatch-does-not-propagate-permissionsallow-from-clcconfigyml-to-worktree-settings`) is a direct consequence — `seed_defaults` is called with `worker_perm_defaults` from config, but nothing verifies the values written to the settings file match what was configured.
3. Workers dispatched with incorrect permissions either get over-broad access (security gap) or under-broad access (workers stall requesting permissions the config already granted). Both failure modes have been observed in production coordinator runs.

## Open Questions

- Can dispatch logic be tested without actually spawning a `claude` process? The FIFO and PID-file infrastructure could be tested in isolation if `spawn_agent_process` is separated from `dispatch`.
- Should `cleanup_stale_worktree` have its own tests, given it modifies git refs and tisket status?
- What's the right boundary: unit tests on individual helpers, or an integration test that runs dispatch against a temp git repo with a tisket?

## Why It Matters

Dispatch is the entry point for all autonomous worker operation. Without tests, every change to the dispatch path — permission seeding, worktree setup, cleanup logic — ships with zero automated verification. The permissions bug alone causes real workflow failures in coordinator runs.
