---
title: "split standalone tools into their own git repos"
status: todo
priority: 1
assignee:
labels: [refactor]
depends_on: []
created: "2026-05-04T00:47:42Z"
updated: "2026-05-04T00:47:42Z"
---

Split tisket, missouri, almanac, belmont, zettel, mdstore, claude-code into their own git repos. Codelikecody will consume them via git deps. Order: leaf crates first (mdstore, claude-code, belmont, almanac), then dependents (tisket, zettel, missouri).

## Why
Each tool is independently useful. Separate repos enable independent release cycles, contributor scopes, and GitHub issue trackers. Reduces accidental coupling.

## Plan per crate
1. cargo workspace member → its own git repo
2. Preserve git history (git subtree split)
3. Push to new repo on GitHub
4. Replace workspace member with [dependencies] git = ... in consumers
5. cargo build --workspace, cargo test --workspace, zero warnings

## Out of scope
- clc, clc-sdk, clc-api, clc-web stay in this repo (orchestration core)
- crates.io publishing (later)
