---
title: "tisket CLI gaps: body flag, close project flag, slug predictability"
status: done
priority: 3
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: 2026-03-05T04:15:00Z
updated: "2026-03-05T04:31:53Z"
---

Spun out from agent-ergonomics tracking tisket. These are specific tisket CLI gaps that agents hit repeatedly.

## Items

### `tisket issue create` needs a `--body` flag
Agents try `tisket issue create -t "title" -b "body"` every time. Neither flag exists. Title is positional-only, body requires creating the issue then appending to the file. Add `-b`/`--body` (inline string) and `--body-file` (read from path). Consider stdin pipe support too.

### `tisket issue close` should accept `-p`/`--project`
`create` takes `-p` but `close` doesn't. Inconsistent — agents guess wrong, then have to check `--help`.

### Slug generation drops punctuation unpredictably
`clc.yaml` becomes `clcyaml` not `clc-yaml`. Agents can't predict what the ID will be. The slugifier should be more predictable — replace punctuation with hyphens, collapse runs.

### `tisket issue list` needs `--label` filter
The coordinator already filters by label internally, but `tisket issue list --label foo` doesn't work. The flag exists in the coordinator code but not the tisket CLI.

### `tisket issue edit --label` sets empty array
Running `tisket issue edit -l tisket <id>` resulted in `labels: []` instead of `labels: [tisket]`. The edit command's label handling is broken.
