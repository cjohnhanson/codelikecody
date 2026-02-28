---
title: "clc done must verify clean working tree before finalizing"
status: todo
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-02-28T20:46:15Z"
updated: "2026-02-28T20:46:15Z"
---

## Problem

A worker reached "done" phase and closed its tisket, but its actual code changes were never committed. When the branch was landed, only the tisket metadata made it to main — the implementation evaporated.

`clc done` currently:
1. Advances phase to done
2. Closes the tisket
3. Commits the phase/tisket changes

It does NOT check whether the working tree has uncommitted changes. A worker that forgets to `git add` + `git commit` its implementation will have `clc done` succeed, marking the work as complete while the code sits in the working tree and gets discarded when the worktree is removed.

## Observed

Worker `context-compaction-drops-operational-knowledge-workers-forget-project-tooling` fixed 3 occurrences of `missouri run` → `clc missouri run` in prime text. Worker ran `clc done`, tisket closed, phase set to done. But the code changes were never committed. On `clc land`, only the finalize commit (tisket closure) was merged. The actual fix was lost.

## Fix

`clc done` should refuse to finalize if there are unstaged or uncommitted changes in the worktree (excluding `.clc/` and `.claude/` which are ephemeral). Error message should tell the worker to commit its changes first.
