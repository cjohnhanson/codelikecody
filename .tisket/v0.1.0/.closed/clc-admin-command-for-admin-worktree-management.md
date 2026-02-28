---
title: "clc admin command for admin worktree management"
status: done
priority: 2
assignee:
labels: [clc]
depends_on: []
created: 2026-02-26T04:15:52Z
updated: "2026-02-28T05:58:50Z"
---

Tisket management, config changes, docs — admin work that doesn't belong on
feature branches or trunk. Currently there's no dedicated place for it, so it
gets committed wherever the session happens to be.

`clc admin` creates or switches to a long-lived admin worktree (`clc-admin`
branch). Unlike `clc pickup` which creates per-tisket branches, the admin
worktree is persistent and shared across sessions.

Behavior:
- If `.worktrees/clc-admin` exists, switch to it
- If not, create worktree + branch from current trunk HEAD
- No phase enforcement — admin work doesn't follow the TDD cycle
- Guard should allow tisket file edits, config edits, doc edits
- Hooks on the admin branch should recognize the context and adjust
  prime text accordingly (no "pick up a tisket" directives)

The admin worktree merges to trunk frequently — small commits, not
batched with feature work.
