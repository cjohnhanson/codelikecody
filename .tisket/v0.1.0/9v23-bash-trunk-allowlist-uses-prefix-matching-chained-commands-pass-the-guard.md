---
title: "bash trunk allowlist uses prefix matching — chained commands pass the guard"
status: todo
priority:
assignee:
labels: [clc, security, guard, auto]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-04-03T18:33:43Z"
---

## Problem

1. The trunk bash guard should only allow specific read-only commands, blocking anything that could modify the working tree.
2. `check_bash_allowlist` in `clc/src/guard.rs` (lines 185-208) checks whether a trimmed command `starts_with` any entry in `BASH_ALLOWLIST` (lines 18-47). This is pure prefix matching — `"git "` matches `"git push --force"`, `"ls"` matches `"ls; rm -rf /"`, and `"cat "` matches `"cat /dev/null; echo pwned > main.rs"`. Shell operators (`;`, `&&`, `||`, `|`, backticks, `$()`) are not considered. The allowlist includes entries like `"cargo build"` which also matches `"cargo build && rm -rf .git"`.
3. Any agent operating on the main branch can execute arbitrary write commands by chaining them after an allowed prefix. The trunk guard provides the illusion of protection without actually constraining behavior.

## Open Questions

- Should the guard parse the command to detect shell chaining operators (`;`, `&&`, `||`, `|`)?
- Should it reject commands containing these operators entirely, or split and validate each sub-command?
- Is there a simpler model — like rejecting any command longer than a single simple invocation on trunk?

## Why It Matters

The trunk guard is the only thing preventing an agent on the main branch from making destructive changes. Prefix matching makes it trivially bypassable: `git status; echo malicious > src/main.rs` passes the `"git "` prefix check.
