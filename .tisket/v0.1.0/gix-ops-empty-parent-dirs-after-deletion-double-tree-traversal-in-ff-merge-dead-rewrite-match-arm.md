---
title: "gix_ops: empty parent dirs after deletion, double tree traversal in ff_merge, dead Rewrite match arm"
status: in_progress
priority:
assignee:
labels: [clc, review-finding]
depends_on: []
created: 2026-03-01T13:15:50Z
updated: "2026-03-01T16:51:48Z"
---

Non-blocking review findings from admin and clean-tree branch reviews:

1. **Empty parent dirs after deletion** (gix_ops.rs, ff_merge): When files are deleted during ff_merge, empty parent directories are left behind. After deleting `.tisket/v0.1.0/test-feature.md`, the `.tisket/v0.1.0/` directory may remain empty.

2. **Double tree traversal in ff_merge** (gix_ops.rs): `collect_tree_blobs` traverses the entire tree twice (once for old, once for new) to compute the set difference for deletion. For large repos this could be slow. A single-pass diff approach would be more efficient.

3. **Dead `Rewrite` match arm** (gix_ops.rs, has_relevant_uncommitted_changes): With `TrackRenames::Disabled`, the `ChangeRef::Rewrite` variant never fires. The arm only checks `source_location`, not destination. If rename tracking were ever enabled, a rename from `.clc/foo` to `src/bar` would be incorrectly filtered. Harmless today but misleading.
