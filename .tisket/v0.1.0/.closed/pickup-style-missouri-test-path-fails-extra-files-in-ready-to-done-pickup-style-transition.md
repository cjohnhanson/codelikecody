---
title: "pickup-style Missouri test path fails: extra files in ready-to-done-pickup-style transition"
status: done
priority:
assignee:
labels: [clc, review-finding]
depends_on: []
created: 2026-03-01T13:22:14Z
updated: "2026-03-01T14:51:27Z"
---

The `ready-to-done-pickup-style` test path (added by the admin-and-tisket-operations-should-never-dirty-main worker) fails on main after merge.

The `initialized → ready-to-done-pickup-style` transition reports extra files: `.claude`, `.claude/settings.local.json`, `Cargo.toml`, `src`, `src/main.rs`. These are expected project files that exist in the source state but the target state fixture doesn't account for.

12/13 clc Missouri paths pass; this is the only failure.
