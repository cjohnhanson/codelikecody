---
title: "Trunk protection and commit discipline"
status: in_progress
priority:
assignee:
labels: [architecture, feature]
depends_on: []
created: 2026-02-25T00:00:00Z
updated: "2026-02-26T13:24:35Z"
---

Nothing should ever change directly on trunk (main/master). All work — features,
admin, tisket management, even one-off chores — happens in dedicated branches and
worktrees. This makes git a reliable store of state and makes prek runs useful
checkpoints.

## Principles

- Trunk is immutable except via merge
- Admin and tisket work gets a dedicated branch (e.g., `clc-admin` or
  epic-specific branches)
- Commits in worktrees should be frequent — low threshold, checkpoint often
- prek runs on each commit are a feature, not overhead
- `clc pickup` already creates worktrees; extend this to admin/non-feature work

## Hook enforcement

- PreToolUse on main: already blocks write tools (Edit, Write, Bash)
- Extend to block `git commit` on main via Bash content inspection, or better:
  block all Bash on main except read-only commands
- SessionStart on main: prime should strongly direct toward picking up work or
  creating an admin worktree
- Stop hook: if on main with uncommitted changes, warn (shouldn't happen but
  defensive)

## Commit discipline

- clc could expose `clc commit` that stages, commits, and runs prek in one step
- UserPromptSubmit reinforcement could include "uncommitted changes: N files"
  to keep it visible
- Commit nudging should be smart, not just "N edits since last commit":
  - Edit count is the dumb-but-works v1 starting point
  - Better signals: a test just went green, a phase transition happened, a
    logical unit of work completed, the agent is about to context-switch to a
    different file cluster
  - Phase system already provides some of this — green transition is a natural
    commit point
  - Missouri test results changing (pass→fail, fail→pass) are meaningful
    checkpoint moments
  - Hard to detect "logical unit complete" without understanding intent, but
    the workflow structure gives proxies for it

## Branch types

- Feature branches: `<tisket-id>` — created by `clc pickup`
- Admin branches: `clc-admin` or `admin/<description>` — for tisket management,
  config changes, docs
- Epic branches: `epic/<name>` — for coordinated multi-tisket work

## Open questions

- Should `clc pickup` support a `--admin` flag for non-tisket work?
- How does merging back to trunk work? `clc done` for features, but admin
  branches may be long-lived
- Should there be a `clc admin` subcommand that creates/switches to the admin
  worktree?
- Commit frequency nudging — count-based? Time-based? Just after every write?
