---
title: "clc pickup"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init, tisket-integration, git-state-detection]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

`clc pickup <tisket-id>` does the full workflow kickoff:
1. Verify tisket is in "todo" and all depends_on are done
2. Create git worktree + branch named after the tisket
3. Set tisket status to in_progress
4. Initialize `.clc/state` in the worktree with phase=tests-unwritten

## Missouri tests

State: ready-to-pickup (initialized project, tisket in "todo", deps satisfied)
Transition: `clc pickup <id>` → picked-up
State: picked-up

Assertions:
- Git branch exists with tisket-derived name
- Worktree directory created
- Tisket status changed to in_progress
- `.clc/state` exists in worktree with phase=tests-unwritten
- `clc pickup` fails if tisket has unresolved depends_on
- `clc pickup` fails if tisket is not in "todo" status
