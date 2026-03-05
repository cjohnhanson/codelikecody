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

### Session 1 — Tests written

Tests added to existing missouri states:
- `has-issue`: JSON output assertions (show --format json, list --format json, show --field), body manipulation assertions (edit --body, --append), move error assertions
- `has-issue-with-body`: JSON output with body/scratch, body replace/append assertions
- `has-many-issues`: JSON list with many items, show --format json with all fields, --field with rich metadata

New missouri states created:
- `issue-moved-to-default`: after moving fix-the-widget from bugs to default project
- `issue-with-set-body`: after `edit --body 'Widget needs fixing urgently'`
- `issue-with-appended-body`: after appending 'Check the save handler' to set body

Key implementation notes:
- `cli.rs` IssueShowArgs needs `--format` and `--field` flags
- `cli.rs` IssueListArgs needs `--format` flag
- `cli.rs` IssueEditArgs needs `--body` and `--append` flags
- Need new `IssueCommand::Move` variant with id + --project flag
- `repo.rs` edit_issue() currently takes status/assignee/due_date — needs body/append params
- `repo.rs` needs new `move_issue()` method
- `jq` added to test packages in project missouri.yml
- `IssueFrontmatter` doesn't derive Serialize — will need it for JSON output
