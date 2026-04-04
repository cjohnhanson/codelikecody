---
title: "permission grants stored in settings.local.json — encapsulate Claude Code permission logic"
status: discovery
priority:
assignee:
labels: [architecture, clc]
depends_on: []
created: "2026-04-04T13:19:58Z"
updated: "2026-04-04T13:19:58Z"
---

## Problem

In local (worktree) mode, permission grants are stored by writing
directly to .claude/settings.local.json. This couples clc's permission
system to Claude Code's internal settings format. The permission grant
store should be a clc abstraction, not a Claude Code implementation
detail.

In Docker mode, grants go through the coordination database, which is
the right abstraction. Local mode should have something equivalent
rather than reaching into Claude Code's settings file.

## Open Questions

- What should the local permission store look like? A file in
  .clc/worker/? An in-memory store that the hook consults?
- Can the phase guard handle permission evaluation entirely, making
  settings.local.json unnecessary for permission grants?
- What's the migration path? Workers currently expect grants to
  appear in settings.local.json.

## Why It Matters

Coupling to settings.local.json makes the permission system fragile
and Claude Code-specific. If the settings format changes, or if clc
supports non-Claude agents, the permission system breaks.
