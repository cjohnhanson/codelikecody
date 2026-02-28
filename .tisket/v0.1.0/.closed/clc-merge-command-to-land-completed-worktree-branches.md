---
title: "clc merge command to land completed worktree branches"
status: done
priority: 2
assignee:
labels: [clc]
depends_on: []
created: 2026-02-26T04:19:51Z
updated: "2026-02-28T05:58:50Z"
---

From trunk, `clc merge <id>` lands a completed worktree branch with guards.

Pre-merge checks:
- Must be on trunk
- Branch/worktree for `<id>` must exist
- Phase must be `done`
- Tisket must be closed
- No uncommitted changes on the branch
- Missouri tests pass on the branch

On success:
- Merge branch into trunk (fast-forward if possible, merge commit if not)
- Optionally clean up the worktree (`git worktree remove`)
- Optionally delete the branch

On failure:
- Report which check failed, don't merge
- Merge conflicts should be reported clearly — the agent or user resolves
  them on the feature branch, not on trunk

This is the last step of the workflow loop. Without it, `clc done` marks work
complete but nothing lands it on trunk.
