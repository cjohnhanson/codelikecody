---
title: "done.rs and merge.rs have zero unit tests — landing and finalization logic untested"
status: in_progress
priority:
assignee:
labels: [clc, testing, auto]
depends_on: []
created: 2026-04-06T13:22:27Z
updated: "2026-04-06T13:37:51Z"
---

## Problem

done.rs (116 lines) and merge.rs (182 lines) have zero unit tests. These handle finalization and landing — high-risk operations where bugs cause data loss (failed merges, lost commits, dirty tree corruption).

## Tests needed

### done.rs
- Fails when not on a feature branch (trunk protection)
- Fails with dirty working tree
- Fails with untracked files
- Succeeds at terminal phase

### merge.rs
- ff-merge succeeds on clean fast-forward
- ff-merge rejects diverged branches (non-ff)
- Validates meaningful commits exist (not just pickup/finalize)
- Handles missing branch gracefully

## From review agent

Testing Priority 6 from QA review. These files are operational code where bugs have the highest blast radius.

## Scratch Notes
