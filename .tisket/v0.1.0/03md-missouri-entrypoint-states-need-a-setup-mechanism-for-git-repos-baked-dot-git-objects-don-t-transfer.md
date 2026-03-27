---
title: "missouri: entrypoint states need a setup mechanism for git repos — baked dot-git objects don't transfer"
status: discovery
priority: 2
assignee:
labels: [missouri, testing]
depends_on: []
created: "2026-03-27T03:28:10Z"
updated: "2026-03-27T03:28:10Z"
---

## Problem

Entrypoint states with `dot-git/` fixtures don't produce working git repos.
The `dot-` convention copies files into `.git/` at runtime, but git object
hashes depend on content + timestamps, and downstream transitions that run
git commands (checkout, commit, branch) fail because the baked objects don't
form a coherent repo in the temp directory.

Tried baking a real `.git/` (14 objects, 156K) into `ready-to-pickup` as
an entrypoint. The fixture files were correct but downstream transitions
like `clc pickup` and `clc dispatch` failed — 11 of 39 paths broke.

The `initialized` entrypoint works because it only has a minimal `dot-git/`
(HEAD + config) — enough for `git rev-parse --git-dir` but not enough for
actual git operations. Its downstream transitions create the real repo.

## Open Questions

- Should missouri support a `setup` block on individual states (not just
  project-level)? An entrypoint could run `git init && git commit` as its
  setup, producing a real repo in the temp dir before assertions/transitions.
- Could the `dot-git/` convention be extended to run `git init` and replay
  a script instead of copying raw objects?
- Is there a way to make git objects portable? The issue might be specific
  to index files containing absolute paths or pack references.

## Why It Matters

`ready-to-pickup` is a shared prefix for 10 paths (pickup, dispatch,
worker, escalation). Each path currently re-runs a 23-second git setup.
Making it an entrypoint would save ~200 seconds of CPU time per full
suite run. The cascading entrypoint boundary fix is already landed —
the path enumeration is correct, just the fixture mechanism is missing.
