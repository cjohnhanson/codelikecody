---
title: "Agent ergonomics: track CLI friction points discovered during autonomous operation"
status: in_progress
priority:
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: 2026-03-01T13:06:31Z
updated: "2026-03-01T15:40:33Z"
---

Tracking tisket for CLI friction points hit during autonomous agent operation. Every time an agent tries a command that doesn't exist, uses a flag wrong, or has to work around a missing feature, log it here. These are real UX signals — if the agent guesses wrong, the interface is probably unintuitive.

This is an ongoing collection, not a single fix. Individual items can be spun out into their own tiskets when they're worth addressing.

## Observed friction

### tisket issue create: no body flag
- **Date**: 2026-03-01
- **What happened**: Agent tried `tisket issue create -t "title" -b "body"`. Neither `-t` nor `-b` exist. Title is positional-only, body has no flag at all.
- **Workaround**: Create the issue, then `cat >>` the body into the markdown file.
- **Expected**: `-b` or `--body` flag, or `--body-file`, or stdin pipe support.

### clc worker log: --tail doesn't exist
- **Date**: 2026-03-01
- **What happened**: Agent tried `clc worker <id> log --tail 5`. The flag is `--lines`, not `--tail`.
- **Workaround**: Checked `--help`, used `--lines` instead.
- **Expected**: Either `--tail` as an alias, or the flag name being more discoverable.
