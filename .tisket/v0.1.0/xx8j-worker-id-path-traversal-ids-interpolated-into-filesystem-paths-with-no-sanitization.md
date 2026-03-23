---
title: "worker ID path traversal — IDs interpolated into filesystem paths with no sanitization"
status: todo
priority:
assignee:
labels: [clc, security]
depends_on: []
created: "2026-03-23T03:12:04Z"
updated: "2026-03-23T03:12:04Z"
---

## Problem

1. Worker IDs used in filesystem paths should be constrained to safe values that cannot escape the intended directory structure.
2. `worker_dir_for` and `working_dir_for` in `clc/src/worker.rs` (lines 733-755) interpolate the `id` parameter directly into `Path::join` calls (`project_dir.join(".worktrees").join(id)`) with no validation. The `id` originates from CLI arguments as a raw `String` (e.g., `clc dispatch <id>`, `clc worker <id> stop`, `clc permissions grant <id>`). While tisket issues pass through `slugify()` in `mdstore/src/slug.rs` — which replaces non-alphanumeric characters with hyphens, preventing `../` — the CLI accepts arbitrary strings. A caller invoking `clc worker "../../../tmp/evil" stop` or `clc permissions grant "../../../etc" "Read"` would construct paths outside `.worktrees/`.
3. `worker_dir_for` with a traversal ID resolves to arbitrary filesystem locations. `prune_workers` (line 69) calls `fs::remove_dir_all` on the resolved path. `permissions::grant` (line 72) calls `working_dir_for` and writes to `.claude/settings.local.json` relative to the result. The `cursor_path` helper (line 602) writes into `.clc/workers/{id}/cursor`, also unvalidated.

## Open Questions

- Does `clc dispatch` always go through tisket's `slugify`, or can a user pass a raw ID that bypasses slugification?
- Are there other entry points (coordinator auto-dispatch, resume, recover) that accept unsanitized IDs?
- Should validation happen once at the CLI boundary, or defensively in `worker_dir_for`/`working_dir_for`?

## Why It Matters

Without path validation, any code path that accepts a worker ID from external input and passes it to `worker_dir_for` can read, write, or delete files at arbitrary filesystem locations relative to the project root.
