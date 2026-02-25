---
title: "Untracked mode"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

`clc init --untracked` mode. All clc/tisket/missouri artifacts get gitignored so
the workflow is invisible to collaborators. Config still needs to be project-wide
but can't live in tracked files — needs a storage strategy (`.clc/` gitignored,
or user-level `~/.config/clc/projects/<id>/`).

## Missouri tests

State: bare-project (git repo, no clc)
Transition: `clc init --untracked` → initialized-untracked
State: initialized-untracked

Assertions:
- `.clc/` exists
- `.claude/settings.local.json` exists
- `.gitignore` (or relevant ignore mechanism) contains `.clc/` entry
- clc artifacts don't show up in `git status`
