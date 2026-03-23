---
title: "install instructions inconsistent across getting-started pages — three different approaches"
status: discovery
priority:
assignee:
labels: [docs, accuracy]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

Each tool's getting-started page should give consistent install instructions since they're all built from the same workspace. The three pages diverge: clc/docs/getting-started.md uses `git clone` + `cargo build --workspace` + manual PATH export. missouri/docs/getting-started.md uses `cargo install --path missouri` (or `cargo build -p missouri` within the workspace). tisket/docs/getting-started.md uses `cargo build --release -p tisket` and tells the user to manually put the binary on PATH. A reader following one page gets a different binary location (target/debug vs target/release), a different build scope (workspace vs single package), and a different install method (PATH export vs manual copy vs cargo install).

## Open Questions

- Is `cargo install --path` the preferred approach, or is the workspace build + PATH export canonical?
- Should there be a single top-level install page that all three reference?
- Is `--release` intentional for tisket or an inconsistency with the other two using debug builds?

## Why It Matters

A user who installs via one page and then reads another will have a different setup than expected. An agent following missouri's instructions won't have clc or tisket available, and vice versa.
