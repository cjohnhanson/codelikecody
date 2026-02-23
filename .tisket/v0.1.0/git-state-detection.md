---
title: "Git state detection"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

Detect git state from the working directory: current branch name, whether we're
in a worktree (`.git` is a file vs directory), whether the branch is main/master.
Exposed via `clc status` and used internally by hooks.

## Missouri tests

States representing different git configurations:
- on-main: git repo on main branch
- on-branch: git repo on a feature branch
- in-worktree: git worktree (`.git` is a file)
- not-git: directory that isn't a git repo

Assertions on each state:
- `clc status` (or internal detection) correctly reports branch name
- `clc status` correctly reports worktree vs main repo
- `clc status` correctly reports is_main true/false
