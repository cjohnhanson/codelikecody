---
title: "tisket CLI bug fixes: label edit, slug predictability, body flag, close project flag"
status: in_progress
priority:
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: 2026-03-05T04:30:59Z
updated: "2026-03-05T04:33:04Z"
---

## Items

### `tisket issue edit -l` sets empty labels
Running `tisket issue edit -l tisket <id>` results in `labels: []` instead of `labels: [tisket]`. The label parsing in the edit command is broken — it's not splitting the comma-separated value correctly.

### Slug generation drops punctuation unpredictably
`clc.yaml` becomes `clcyaml` not `clc-yaml`. The slugifier should replace punctuation with hyphens and collapse runs. Note: this is a behavior change — new issues get predictable slugs but existing slugs are unaffected.

### `tisket issue create` needs `--body` flag
Agents try `-b "body text"` every time. Title is positional-only, body requires creating the issue then appending to the file. Add `--body` (inline string) and `--body-file` (read from path).

### `tisket issue close` should accept `-p`/`--project`
`create` takes `-p` but `close` doesn't. Inconsistent — agents guess wrong.

## Verification

- Edit with `-l foo,bar` sets `labels: [foo, bar]`
- Slug for "clc.yaml support" becomes `clc-yaml-support`
- Create with `--body "text"` includes body in the file
- Create with `--body-file <path>` reads body from file
- Close with `-p v0.1.0 <id>` works

## Scratch Notes

### Tests written (phase: tests-written)

**Bug analysis:**
- Label edit: cli.rs:321 calls `repo.edit_issue()` but never passes labels/title/priority/assignee. The `edit_issue` method in repo.rs:509 doesn't accept those params either. Two fixes needed: pass args through cli.rs, expand edit_issue signature.
- Slug: slug.rs:49 only treats ` `, `-`, `_` as separators. All other punctuation (`.`, `,`, `!`, etc.) is silently dropped. Fix: treat any non-alphanumeric char as a separator.
- Body: IssueCreateArgs has no body/body_file fields. Need to add them and wire through to create_issue + serialize_issue.
- Close -p: IssueCloseArgs has no project field. Need to add it and use it in resolve_id lookup.

**Missouri test states created:**
- `issue-labels-edited` — target for `tisket issue edit -l foo,bar`
- `issue-closed-via-project` — target for `tisket issue close -p bugs`
- `has-issue-with-body-flag` — target for `--body` create
- `has-issue-with-body-file` — target for `--body-file` create

**Helper scripts created:**
- `verify-slug-punctuation` — creates issue, checks slug matches expected
- `create-issue-with-body-file` — writes temp file, creates issue with --body-file

**Unit tests added:**
- `slug.rs::punctuation_replaced_with_hyphens` — 5 assertions for punctuation → hyphen behavior

**Key files for implementation:**
- tisket/src/cli.rs — IssueCreateArgs, IssueCloseArgs, IssueEditArgs, run_command
- tisket/src/slug.rs — slugify function
- tisket/src/repo.rs — edit_issue, close_issue, create_issue, CreateIssueOptions
