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

## Session: coordinator-run-scratch-notes-tisket (2026-03-02)

- Worker completed scratch-notes tisket successfully, but `clc land` failed because main advanced (tisket scoping commits) while the branch was in flight. `clc land` requires fast-forward, so the branch needs rebasing.
- Neither the coordinator nor workers can run `git rebase` — blocked by both the trunk allowlist (coordinator runs on trunk) and Claude Code's permission system (workers run with `--dangerously-skip-permissions` but that doesn't cover rebase).
- Coordinator got stuck and died trying to resolve this. No automatic recovery path.
- **Gap**: `clc land` should either rebase automatically or the coordinator needs permission to rebase worker branches. This is a fundamental issue for any coordinator run where main advances during worker execution.

## Session: clc-up-epic-dispatch (2026-03-10)

- **ToolSearch blocked on trunk**: `clc hook` blocks ToolSearch (fetching deferred tool schemas) on trunk. ToolSearch is purely read-only — it fetches JSON schemas so tools can be invoked. No write risk. Should be allowlisted alongside Read/Glob/Grep.

- **`clc worker check` stale read offset**: Workers actively producing output (stdout.jsonl growing, process alive) but `clc worker check` returns "no new activity" or "no worker output." The check command seems to lose track of its read position, making it useless for monitoring long-running workers. Had to fall back to raw `tail`/`jq` on stdout.jsonl.

- **Worker can't commit — permission denied for Bash(git)**: xoh6 worker completed all implementation and tests (250 unit tests, 2 missouri paths) but couldn't finalize because Bash permissions for git commands weren't granted. Worker produced a result message asking for permission instead of completing. Had to finalize manually from admin session.

- **`clc done` from green phase fails**: Error "phase must be 'done' to finalize, currently 'green'." But `clc done` is supposed to *advance* to done. The error message is contradictory — if you need to already be at 'done' to run `clc done`, how do you get to done? Workaround: manually write `phase: done` to `.clc/state`, then `clc done` succeeds.

## Session: mitmproxy-network-mocking coordinator run (2026-03-16)

- **Permission grant doesn't unblock worker automatically**: Worker requested permission to run `nix develop`. Coordinator granted it via `clc permissions grant`. Worker stayed idle — "no new activity." Had to manually `clc worker send` a nudge message telling the worker the permission was granted before it resumed. The grant should either automatically resume the worker or inject a message into its stdin pipe so it knows the permission was granted without human intervention.

- **Permission mismatch on command form**: Worker first requested `nix develop /full/path/to/flake#dev --command cargo test`. Permission was granted for "nix develop". Worker then tried the shorter form `nix develop --command cargo test -p missouri` which was denied because the permission string didn't match. Had to grant a second, more specific permission. Permission matching should be more flexible — granting "nix develop" should cover all `nix develop` invocations, not require exact command string matches.

