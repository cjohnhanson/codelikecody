---
title: "Missouri tests for admin worktree guard permissiveness"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-18T02:34:53Z
updated: "2026-03-18T02:46:58Z"
---

## Scratch Notes

Added 5 assertions to ready-to-pickup/.missouri/missouri.yml:
- admin guard allows Edit in src without phase
- admin guard allows Write without phase
- admin guard allows Stop without phase
- admin guard allows unrestricted Bash
- admin prime shows (admin) annotation

These follow the same pattern as phase-tests-unwritten guard assertions
(pipe JSON hook events to `clc hook`, check exit code).

Note: the full missouri test suite has pre-existing failures at the
`initialized` transition step. These are unrelated to this change —
`clc init` works fine when tested manually with the built binary.
