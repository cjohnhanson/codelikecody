---
title: "Expressive tisket CLI: all operations without file reads or writes"
status: todo
priority: 3
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: "2026-03-05T04:15:00Z"
updated: "2026-03-05T04:15:00Z"
---

## Problem

Many tisket operations currently require reading or writing the markdown file directly. On trunk, file writes are blocked by clc hooks, so agents resort to sed/cat workarounds. Even in worktrees, having to parse frontmatter manually is error-prone.

Everything should be doable through the CLI.

## Missing operations

### Read operations
- `tisket issue show <id> --field tags` — extract a specific field value
- `tisket issue show <id> --format json` — machine-readable output
- `tisket issue list --label <label>` — filter by label
- `tisket issue list --where key=value` — filter by tag (depends on tags tisket)
- `tisket issue list --format json` — machine-readable list output

### Write operations
- `tisket issue edit --body "text"` — replace body content
- `tisket issue edit --append "text"` — append to body (the `cat >>` workaround)
- `tisket issue edit --tag key=value` — set a tag
- `tisket issue edit --untag key` — remove a tag
- `tisket issue edit --add-label foo` — add without replacing all labels
- `tisket issue edit --remove-label foo` — remove a single label
- `tisket issue comment <id> "text"` — append a timestamped note (scratch notes use case)

### Lifecycle operations
- `tisket issue move <id> --project <project>` — move between projects

## Principle

If an agent needs to `cat` or `sed` a tisket file, the CLI is missing a feature. The file format is the storage layer; the CLI is the interface.
