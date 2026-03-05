---
title: "Machine-readable output and remaining CLI expressiveness"
status: in_progress
priority: 4
assignee:
labels: [tisket]
depends_on: []
created: 2026-03-05T04:31:04Z
updated: "2026-03-05T04:33:05Z"
---

## Items

### JSON output
- `tisket issue show <id> --format json` — frontmatter + body as JSON
- `tisket issue list --format json` — array of issue objects
- `tisket issue show <id> --field <name>` — extract a single field value

### Body manipulation on edit
- `tisket issue edit <id> --body "text"` — replace entire body below frontmatter
- `tisket issue edit <id> --append "text"` — append to body

### Move between projects
- `tisket issue move <id> --project <target>` — move issue file to a different project directory

## Notes

These are lower-priority conveniences. Agents can work around all of them with existing commands. The scratch subcommand and bug fixes tiskets cover the high-friction gaps.

## Scratch Notes
