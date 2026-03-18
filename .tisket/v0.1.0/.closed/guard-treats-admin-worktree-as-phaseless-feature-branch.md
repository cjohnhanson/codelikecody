---
title: "Guard treats admin worktree as phaseless feature branch"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-18T02:20:49Z
updated: "2026-03-18T02:31:58Z"
---

## Problem

`clc admin` creates a worktree on branch `clc-admin` with no phase set. The guard sees `is_main: false` and `phase: None`, defaults to `TestsUnwritten`, and blocks all non-test edits. `check_stop` also blocks exit. The admin worktree is unusable.

## Fix

1. Add `admin_branch` to config (default `"clc-admin"`)
2. Add `is_admin` to `GitState`, derived from config during `git::detect`
3. Guard treats `is_admin` as fully permissive: no phase enforcement, no stop blocking
4. `admin.rs` reads admin branch name from config instead of hardcoding

## Related

- stop-hook-enforcement-for-coordinator-and-admin-sessions (discovery)

## Scratch Notes

- Files to change: config.rs, git.rs, guard.rs, admin.rs, hook.rs (prime text)
- guard.rs:95 — `is_main` check needs `is_admin` sibling
- guard.rs:59 — `check_stop` needs `is_admin` passthrough
- admin.rs:6 — hardcoded `ADMIN_BRANCH` const needs to come from config
- config.rs — add `admin_branch` field to `ProjectSection` and `Config`
- git.rs:16 — `detect` needs `admin_branch` param
- hook.rs uses `git::detect(cwd, &cfg.main_branch)` — needs to pass admin_branch too
- main.rs:252 — hardcoded print of "clc-admin" path
