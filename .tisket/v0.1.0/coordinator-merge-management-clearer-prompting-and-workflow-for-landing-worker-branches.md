---
title: "Coordinator merge management: clearer prompting and workflow for landing worker branches"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-02T13:54:06Z"
updated: "2026-03-02T13:54:06Z"
---

Landing worker branches is a significant part of the coordinator's job but the current prompt barely covers it. The lifecycle says "Land completed work: `clc land <id>`" and that's it. In practice, landing involves:

1. Verifying the worker reached `done` phase
2. Checking that tests pass (cargo test + missouri)
3. Handling the common case where main has advanced (rebase needed — separate tisket for auto-rebase)
4. Dealing with land failures (stale worktrees, tisket not closed, merge conflicts)
5. Cleaning up after successful landing (worktree removal, branch deletion — `clc land` handles this)
6. Moving on to the next tisket

The coordinator currently gets stuck on landing failures because the prompt doesn't give it a playbook. It either asks the user for help or spins.

## What the prompt needs

- **Pre-land checklist**: before running `clc land`, verify worker status with `clc worker <id> check`, confirm phase is done
- **Land failure handling**: if `clc land` fails, read the error message. Common failures and what to do:
  - "not a descendant of HEAD" → main advanced, ask user to rebase (until auto-rebase lands)
  - "tisket not closed" → worker didn't run `clc done` properly, resume and instruct
  - "phase is not done" → worker stopped early, resume and instruct
  - "working tree has uncommitted changes" → something is dirty on trunk, investigate
- **Post-land flow**: after successful landing, immediately check for more todo tiskets and dispatch
- **Stale worker cleanup**: if a worker is dead (PID not alive) but not in done phase, that's a failed run. Log it, clean up, maybe re-dispatch.

## This is a prompt change

Requires user approval per CLAUDE.md before writing to file.
