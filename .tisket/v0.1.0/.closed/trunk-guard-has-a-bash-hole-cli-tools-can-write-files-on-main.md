---
title: "Trunk guard has a Bash hole — CLI tools can write files on main"
status: done
priority: 1
assignee:
labels: [clc, guard]
depends_on: []
created: 2026-02-26T04:26:19Z
updated: "2026-02-28T06:10:46Z"
---

The trunk guard blocks Edit/Write/NotebookEdit on main but allows Bash
unrestricted. Any CLI tool that writes files (`tisket issue create`, `cargo init`,
`echo > file`) bypasses trunk protection entirely.

Two complementary fixes:

1. **Bash allowlist on trunk.** Block Bash by default on main, allow commands
   matching safe prefixes: `git`, `cargo test`, `cargo clippy`, `cargo fmt --check`,
   `tisket issue list`, `tisket issue show`, `tisket search`, `clc`, `missouri run`,
   `ls`, `pwd`, `which`. If the guard can't determine safety, block and explain.
   Conservative — false positives are better than writes on trunk.

2. **Workflow routing makes it moot.** With `clc admin` providing a dedicated
   worktree for admin work, the agent should never need to write on trunk. The
   SessionStart prime text on main should direct toward `clc pickup` or `clc admin`.
   The allowlist is the backstop; the workflow is the primary enforcement.
