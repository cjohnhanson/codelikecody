---
title: "Agent ergonomics: track CLI friction points discovered during autonomous operation"
status: discovery
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

### Session 2026-03-01 (coordinator implementation)

- `tisket issue create -t "title"` fails — title is positional, not `-t` flag. Agents guess `-t` because every other CLI uses it.
- `tisket issue close -p v0.1.0 <id>` fails — `-p` not a valid flag for close. Inconsistent with `create` which takes `-p`.
- `git add ".tisket/v0.1.0/file.md"` blocked by git-add-validator — hook sees `.tisket/` as a directory add. Agents hit this repeatedly and have to use `git rm` for deletions or find other workarounds.
- `clc phase green` — no such subcommand. Phase advancement isn't exposed as a CLI command, agents have to manually write to `.clc/state`.

## Session: permission-request-system (2026-03-01)

- `tisket issue create` has no `-b`/`--body` flag — body must be appended to the file after creation
- `tisket issue create` title becomes a slug that drops punctuation unpredictably (clc.yaml → clcyaml)
- git-add-validator false positive: `git add clc/src/permissions.rs` blocked as "adding a directory: clc" — had to use `./clc/src/permissions.rs` with leading `./`
