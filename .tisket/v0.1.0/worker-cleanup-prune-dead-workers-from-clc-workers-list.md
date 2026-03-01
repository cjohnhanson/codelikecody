---
title: "Worker cleanup — prune dead workers from clc workers list"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T06:02:07Z
updated: "2026-03-01T02:56:55Z"
---

`clc workers` lists every worktree with a `.clc/worker/` directory, including
dead workers that were stopped but never landed. These accumulate over time.

Options:
- `clc workers --prune` or `clc workers clean` to remove worker state files
  from dead workers (delete `.clc/worker/` in worktrees where pid is dead)
- `clc workers` could hide dead workers by default, show with `--all`
- `land` should clean up worker state as part of its flow
- Coordinator cursor files (`.clc/workers/<id>/cursor` on trunk) also need cleanup
