---
title: "Missouri workspace mode: stale doc comment, missing --record support, no list test"
status: discovery
priority:
assignee:
labels: [missouri, review-finding]
depends_on: []
created: 2026-03-01T13:15:49Z
updated: "2026-03-01T16:51:49Z"
---

Non-blocking review findings from workspace mode branch review:

1. **Stale doc comment on `member_label`** (cli.rs ~line 399): Says "Uses the directory basename, or the last two components if the basename is generic." Actual implementation does `strip_prefix(workspace_root)` with fallback to `file_name()`. No "last two components" logic exists.

2. **`--record` silently ignored in workspace mode**: Line 454 hard-codes `recording: None`. If `--record` is passed with a workspace config, nothing happens and no warning is emitted. Should either support workspace recording or emit an error/warning.

3. **No `list` test for workspace mode**: `run`, `run` with failures, `-C` flag, and `validate` are all tested. `list` in workspace mode is untested.

4. **Code duplication**: `run_workspace_members` body is essentially a copy of the main `Command::Run` handler with a for-loop. Could extract shared `run_single_suite()`.
