---
title: "Stop hook enforcement for coordinator and admin sessions"
status: cancelled
priority:
assignee:
labels: [clc]
depends_on: []
created: "2026-03-23T02:31:43Z"
updated: "2026-03-23T02:31:43Z"
---

## Problem

The coordinator runs on trunk. `check_stop()` in guard.rs passes through on main — so the coordinator can exit any time the model decides to. No mechanical enforcement keeps it alive.

Workers have stop hook enforcement: they can't exit before reaching `done` phase. The coordinator has no equivalent. Its persistence is entirely prompt-driven, which means context compaction, model whims, or confusion can kill it silently.

## What needs to happen

The stop hook needs awareness of coordinator and admin sessions:

- **Coordinator**: should not be allowed to stop while it has active (non-landed) workers or unprocessed pickable tiskets. Detection could check for the coordinator worker dir or a flag in `.clc/`.
- **Admin**: needs a defined stop policy — maybe always passthrough, maybe something else.

## Design considerations

- The coordinator's "done" condition is different from a worker's — it's done when all dispatched work is landed and no more pickable tiskets remain
- This is about the Stop event in `guard.rs`, not about adding a Rust while-loop
- `check_stop()` currently short-circuits to Passthrough on `is_main` (line 64) — that's the code path that needs to become conditional
