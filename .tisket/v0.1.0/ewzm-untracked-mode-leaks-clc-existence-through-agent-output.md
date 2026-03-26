---
title: "Untracked mode leaks clc existence through agent output"
status: todo
priority: 2
assignee:
labels: [clc]
depends_on: []
created: "2026-03-26T03:06:18Z"
updated: "2026-03-26T03:06:18Z"
---

## Problem

`clc init --untracked` hides files from git via `.git/info/exclude`, but
the agent still references clc in its output. Commit messages mention
`clc done`, PR descriptions reference tisket IDs, code comments suggest
`tisket issue create`. The project's contributors don't know clc is
there and shouldn't find out from the agent's behavior.

Untracked mode means invisible — not just "files hidden" but "no trace
in any contributor-visible output."

## Acceptance Criteria

- [ ] Given `clc init --untracked` was run, when `is_untracked()` is
      called, then it returns true by checking `.git/info/exclude` for
      clc patterns (no `.clc/state` file dependency)
- [ ] Given untracked mode, when SessionStart fires, then the prime
      text includes a directive forbidding mention of clc, tisket,
      missouri, zettel, almanac, belmont, or any clc-specific commands
      in commit messages, PR descriptions, code comments, documentation,
      or any other contributor-visible output
- [ ] Given untracked mode, when the agent uses clc tools internally
      (phase transitions, tisket lookups, etc.), then that usage is not
      suppressed — only external-facing output is constrained
- [ ] Given `write_untracked_state()` in init.rs writes `untracked: true`
      to `.clc/state`, then that code is removed — the git exclude is
      the source of truth, not a state file

## Done When

- `is_untracked(project_dir)` function exists, encapsulated, reads git exclude
- Prime text includes stealth directive when untracked
- `.clc/state` is no longer written or read for untracked detection
- Agent in untracked mode produces clean commit messages with no clc references
