---
title: "Deterministic test result caching via git tree state"
status: discovery
priority:
assignee:
labels: [missouri, research]
depends_on: []
created: "2026-02-25T04:34:02Z"
updated: "2026-02-25T04:34:02Z"
---

Missouri currently has no persistent results — every `clc status` re-runs the
full test suite. v0.1.0 adds basic timestamp-based caching (run counter +
timestamp, no staleness detection). This tisket explores the harder problem:
tying results to actual repository state so staleness is deterministic.

## Problem

Cached test results are only valid for the inputs that produced them. If source
files change, cached "all passing" is a lie. But missouri runs arbitrary shell
commands — it has no explicit dependency graph, so "what are the inputs?" is
unknowable in general.

## Questions to explore

- Can missouri manage a git tree object (or commit) representing the working
  tree state at test time? `git write-tree` + `git stash create` are cheap.
- Is per-file hashing of the test fixture directories enough, or do we need
  to capture the full working tree?
- Should missouri track which files its transitions actually touch (via strace,
  fs-level diffing, or sandboxing) to build an implicit dependency graph?
- Would a simpler "invalidate on any tracked-file change" approach be good
  enough in practice?
- How do untracked files factor in? New source files won't appear in git diff.
- What do other tools do? (Bazel remote cache keying, Nx affected, turborepo
  hash-based caching)
