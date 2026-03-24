---
title: "belmont: file exfiltration — agent can write secrets to files then read them"
status: discovery
priority: 3
assignee:
labels: [belmont, security, epic:ifsy]
depends_on: []
created: 2026-03-24T12:42:23Z
updated: "2026-03-24T12:53:32Z"
---

## Problem

`belmont run -- sh -c 'echo $SECRET > /tmp/leak.txt'` writes the secret value
to disk. A subsequent `cat /tmp/leak.txt` via the Read or Bash tool brings the
value into the agent's context unscrubbed. The PTY scrubbing only covers
stdout/stderr of the subprocess — file writes bypass it entirely.

This is not unique to belmont. 1Password's `op run` has the same limitation.
Once values are in the subprocess environment, the subprocess can write them
anywhere. `op` considers this out of scope — the protection is about what the
*caller* sees, not what the subprocess does.

The difference for belmont: the caller is an agent, and the agent reading
files is a normal part of its workflow. With `op`, the caller is a human who
can already see secrets. With belmont, the caller specifically should not.

## Possible mitigations

- **Prime text.** Already says "never read files that contain secret values."
  Could be more specific: "never read files written by commands run through
  `belmont run`."
- **Tmpdir isolation.** Run the subprocess in a temporary directory overlay
  that's cleaned up after exit. Prevents persistent file writes. Adds
  complexity and may break commands that need to write to the project dir.
- **PostToolUse scanning.** Scan Read/Bash tool responses for known secret
  values. Can't prevent the leak but can warn. See hardening epic (ifsy).
- **Accept the limitation.** Document it clearly. The agent runs locally
  under the human's permission model. If the agent is constructing deliberate
  exfiltration commands, secrets in env vars are the least of the problems.
