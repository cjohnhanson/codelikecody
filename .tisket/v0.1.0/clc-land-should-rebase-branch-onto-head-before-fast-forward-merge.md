---
title: "clc land should rebase branch onto HEAD before fast-forward merge"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-02T06:00:00Z
updated: "2026-03-03T02:08:45Z"
---

`clc land <id>` currently requires the worker branch to be a direct descendant of HEAD (fast-forward only). When main advances during a coordinator run — from tisket status updates, other workers landing, or manual commits — the branch falls behind and `clc land` fails.

The coordinator has no way to fix this: `git rebase` is blocked by the trunk allowlist, and workers can't rebase either.

## Approach: cherry-pick rebase via tree-editor

When `ff_merge` detects the branch is not a descendant of HEAD, rebase the branch commits onto HEAD before the fast-forward. Uses gix's `tree-editor` feature (already enabled) — no new gix features or dependencies needed.

### Algorithm

1. Find the merge base (fork point) between HEAD and branch tip
2. Collect branch commits from fork point to branch tip (oldest first)
3. Compute the set of paths modified between fork point and HEAD ("main's changes")
4. For each branch commit (oldest to newest):
   a. Diff the commit's tree against its parent's tree to get the set of changed entries
   b. **Conflict check**: if any path was modified in BOTH the branch commit AND main's changes → abort with conflict error
   c. Start with the new base tree (HEAD's tree for first commit, previous rebased commit's tree for subsequent)
   d. Use tree-editor to apply each change: upsert for adds/modifications, remove for deletions
   e. Write the new tree
   f. Create a new commit: same message, same author, new tree, new parent (HEAD for first, previous rebased commit for subsequent)
5. Update the branch ref to point to the last rebased commit
6. Now the branch is a descendant of HEAD — proceed with normal fast-forward

### Key details

- **Merge base**: walk ancestors of both HEAD and branch tip, find first common ancestor. gix has `is_ancestor` already in gix_ops — similar traversal.
- **Tree diffing**: compare two trees entry by entry. Entries that differ (by oid or mode) are "changed." gix `tree-editor` + `traverse().breadthfirst` can enumerate entries.
- **Conflict detection is conservative**: if the same path appears in both diff sets, abort. No three-way merge, no conflict resolution. This handles the common case (tisket updates on main, code changes on branch) and fails cleanly on the rare case.
- **Author preservation**: rebased commits should keep the original author and message. Only the tree and parent change.

### Where to implement

In `gix_ops.rs`, add a `rebase_onto_head(project_dir, branch_name)` function. Called from `ff_merge` when `is_ancestor` returns false — try the rebase, then retry the fast-forward.

### What NOT to do

- Do not shell out to git
- Do not enable gix `merge` feature — the tree-editor approach is sufficient
- Do not attempt three-way merge or conflict resolution — abort on any path overlap

## Testing

- Unit test: create repo where main advanced with non-overlapping commits, verify rebase succeeds and ff_merge works
- Unit test: create repo where main and branch both modify the same file, verify rebase aborts with conflict error
- Unit test: multi-commit branch (2-3 commits) rebased correctly preserves all commits with right parents and messages
- Missouri test: coordinator run where main advances during worker execution, landing succeeds without manual intervention

## Observed in

Coordinator run on 2026-03-02: scratch-notes worker completed successfully but couldn't land because main had advanced with tisket scoping commits. Coordinator got stuck, resumed the worker to try rebasing, worker couldn't rebase either. Required manual intervention.
