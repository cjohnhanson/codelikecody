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
