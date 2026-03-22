---
title: "Use bollard for hermetic container-based test execution in missouri"
status: in_progress
priority: 2
assignee:
labels: [missouri, network]
depends_on: []
created: 2026-03-21T18:32:56Z
updated: "2026-03-21T18:33:04Z"
---

## Scratch Notes

### Current state (2026-03-22)

DockerBackend via bollard on main. Containers with network_mode:none,
volume mounts, stdout/stderr capture, transparent HTTPS replay via
mitmdump. Dockerfile support for custom images. All cargo tests passing.

mitm-test Dockerfile committed at missouri/docker/mitm/Dockerfile.
Not yet integrated into test flow.

### What existing clc tests DON'T cover

The clc/tests/missouri/ suite (82 states) uses a stub claude binary
that emits fixed JSON. It tests clc's mechanical behavior (worktrees,
PIDs, phases, permissions) but NOT actual Claude Code behavior. These
workflows assume Claude follows the prime text instructions but never
verify it:

### Behavioral test workflows needed

Each of these would be a recorded flow file from a real Claude session,
replayed hermetially inside a Docker container.

**1. Worker completes a simple tisket (happy path)**
- Record: `clc dispatch` on a tisket with a trivial implementation
- Worker reads tisket, writes tests, implements, runs tests, calls `clc done`
- Verify: tisket closed, phase=done, code committed, tests pass
- This is the most important test — proves the full dispatch→done loop

**2. Worker follows TDD — tests before implementation**
- Record: worker session where tests are written first
- Verify: phase advances tests-unwritten → tests-written → red → implementing → green
- Verify: test files exist before implementation files

**3. Worker handles failing tests**
- Record: worker session where tests fail, worker fixes, re-runs
- Verify: the fix loop works, final state is green

**4. Worker reads and follows prime text**
- Record: worker session on a brand new tisket
- Verify: worker reads scratch notes, writes to scratch notes during work
- Verify: worker uses `clc status` to check phase

**5. Worker respects phase gates**
- Record: worker tries to edit source during tests-unwritten phase
- Verify: PreToolUse hook blocks the edit, worker adjusts

**6. Worker handles permission denials**
- Record: worker requests a permission, gets denied, finds workaround
- Verify: permission escalation message sent to coordinator

**7. Coordinator monitors and lands worker**
- Record: coordinator session that dispatches worker, monitors it, lands
- Verify: full coordinator→worker→land cycle works

**8. Worker commits frequently**
- Record: worker session with multiple commits
- Verify: commits happen at checkpoints, not just at the end

**9. Worker captures discovered work as tiskets**
- Record: worker session where it finds a bug/TODO and creates a tisket
- Verify: tisket created with correct project/labels

**10. Worker handles `clc done` failure**
- Record: worker tries `clc done` with dirty tree, handles the error
- Verify: worker commits, retries, succeeds

### Recording workflow

To record a flow file for any of the above:

1. Start mitmdump in recording mode on a separate port
2. Set HTTPS_PROXY pointing at mitmdump (or use --mode local for
   transparent capture — acceptable for one-off recording since we
   control the environment)
3. Run `clc dispatch <tisket-id>`
4. Worker does its thing, making real API calls through mitmdump
5. Save the flow file in the test fixture

The flow file captures every Anthropic API request/response in order.
Replay serves them back in the same order.

### Test image requirements

The Docker image for replay tests needs:
- claude-code (for the worker process)
- clc, tisket, missouri (the project binaries)
- git (for worktree/commit operations)
- mitmproxy (for the replay infrastructure)
- Standard Unix tools (bash, coreutils, etc)

The user provides a Dockerfile in the test state's .missouri/ directory.
For our tests, this Dockerfile will use nix to build the flake and install
project binaries. The Dockerfile is committed to the repo.

### Open questions

- How to handle non-deterministic parts of API responses (timestamps,
  request IDs, etc) — mitmdump's server_replay matches on URL+method,
  ignoring headers by default. Should be fine.
- How to handle streaming responses — the Anthropic API uses SSE.
  mitmdump captures the full response including streaming chunks.
  Replay serves the same chunks.
- Flow file size — a full worker session might produce a large flow
  file. Need to check if this is manageable in the repo.
- How to update recordings when prompts change — recorded responses
  are specific to the conversation flow. Prompt changes mean re-recording.

### Session 2026-03-22 — Recording attempt

**Recording image built:** `missouri-record` — nixos/nix base with claude-code
2.1.77, mitmproxy 12.2.1, iptables. Project binaries (clc, tisket, missouri)
built separately via `nix develop --command cargo build --release` inside Docker
(64 seconds) and mounted as a volume.

**First recording attempt:**
- Used HTTPS_PROXY mode (not transparent iptables — `su` not in nixos/nix image)
- Worker dispatched (pid 18), ran for 10 minutes
- Status polling returned "unknown" — was running from project root, not worktree
- Flow file was 11GB — captured ALL HTTPS traffic, not just Anthropic API
- Worker never reached "done" status

**Issues to fix for next attempt:**
1. Filter recording to only Anthropic API: `mitmdump --set flow_detail=0 -w file.flow ~d api.anthropic.com` (domain filter)
2. Status polling: run `clc status` from the worker's worktree directory
3. Worker completion: need to detect when claude process exits, not poll forever
4. The HTTPS_PROXY approach requires NODE_EXTRA_CA_CERTS for claude-code's Node.js — which is application-level config. For recording this is acceptable but should be documented.
5. nixos/nix image lacks `su`/`adduser` — mitmuser created via /etc/passwd hack, UID 1000

**What works:**
- missouri-record Docker image builds (claude-code + mitmproxy + iptables + project binaries)
- clc dispatch runs inside the container
- mitmdump captures traffic through HTTPS_PROXY
- Project binaries (clc, tisket, missouri) work inside the container

### Session 2026-03-22 — Recording attempt #2

**Fixes applied:**
- Domain filter `~d api.anthropic.com` to reduce flow file size
- PID-based wait instead of polling (check if worker process is alive)
- Status check from worktree directory
- Git state cleanup between attempts (worktree remove, branch delete, tisket reset)

**Result:**
- Worker dispatched (pid 13), made ~150 API calls over 5 minutes
- Phase stayed at `tests-unwritten` — worker didn't advance phases
- Flow file was **0 bytes** — the domain filter as a positional arg
  prevents mitmdump from writing ANY flows (filter applies to save, not display)
- Worker timed out at 300s

**Root cause of empty flow file:**
The `~d api.anthropic.com` argument on the mitmdump command line acts as a
filter on what flows are processed, not just displayed. With HTTPS_PROXY mode,
all traffic through the proxy IS the target traffic — no filter needed.
Remove the filter expression entirely.

**Root cause of worker not advancing:**
Unknown. The worker was making API calls (visible in mitmdump connection logs)
but never left `tests-unwritten` phase. Possibly:
- The tisket body didn't have enough context for Claude to act on
- The phase gate blocked something and the worker got stuck in a loop
- The `clc dispatch` environment inside Docker is missing something
  (no .claude/CLAUDE.md, no skills, etc.)

### Session 2026-03-22 — Recording attempt #3: SUCCESS

**Auth fix:** `CLAUDE_CODE_OAUTH_TOKEN` (not `ANTHROPIC_AUTH_TOKEN`) is the
correct env var. Token read from `~/.claude/token` mounted as a Docker secret.
`ANTHROPIC_AUTH_TOKEN` authenticates for `claude auth status` but fails with
`401: OAuth authentication is currently not supported` when making actual API
calls through the proxy.

**Result: Full worker lifecycle completed.**
- `tests-unwritten` → `tests-written` (120s)
- `tests-written` → `implementing` (315s)
- `implementing` → `green` (480s)
- `green` → `review-requested` → `done` (525s)
- 234 tool uses, 0 auth errors
- 16MB flow file at `/tmp/record-output/worker-happy-path.flow`
- missouri: 1/1 passing in the worktree
- Tisket closed

**Worker behavior verified:**
- Added `pub fn hello(name: &str) -> String` returning `"Hello, {name}!"`
- Added test `hello_greets_by_name`
- Created missouri tests (3 states)
- Single implementation commit + finalize commit
- Followed TDD: wrote tests first, confirmed red, implemented, confirmed green

**Recording command that works:**
```
docker run --rm --cap-add NET_ADMIN \
  -v ~/.claude/token:/run/secrets/anthropic-token:ro \
  -v /tmp/clc-linux-bin:/clc-bin:ro \
  -v /tmp/record-test:/project \
  -v /tmp/record-output:/recordings \
  missouri-record sh -c '
export CLAUDE_CODE_OAUTH_TOKEN=$(cat /run/secrets/anthropic-token)
export PATH=/clc-bin:$PATH
export HOME=/root
mitmdump --mode regular -p 18080 -w /recordings/flow.flow -q &
sleep 2
cd /project
HTTPS_PROXY=http://127.0.0.1:18080 \
HTTP_PROXY=http://127.0.0.1:18080 \
NODE_EXTRA_CA_CERTS=/root/.mitmproxy/mitmproxy-ca-cert.pem \
clc dispatch <tisket-id>
# wait for worker PID to exit, then kill mitmdump
'
```

### Remaining problems for replay tests

**1. Flow file contains secrets**
The flow file has Authorization bearer tokens in HTTP headers. Cannot be
committed to the repo as-is. Needs a scrubbing step to remove auth headers
before storage.

**2. Flow file size**
16MB uncompressed, 6MB gzipped. Per-recording. Multiple behavioral tests
means multiple recordings. Binary files don't diff well in git.

Options considered:
- Git LFS (adds dependency)
- Commit compressed (bloats repo history)
- Scrub + compress (best balance)
- Generate on demand (defeats hermetic replay purpose)

**3. Container auth for workspaces (general problem)**
`CLAUDE_CODE_OAUTH_TOKEN` works for recording. For replay tests, the
container doesn't need real auth (traffic is replayed). But for clc
workspaces (the general case), agent auth needs to flow into containers.
This is the Belmont problem — Belmont as an LLM gateway would handle
auth injection and secret scrubbing at the network layer.

**4. Scrubbing flow files**
mitmdump has addon support. A scrubbing script should:
- Remove Authorization / X-Api-Key headers from requests
- Remove Set-Cookie headers from responses
- Optionally redact sensitive patterns in bodies
- Preserve enough structure for replay matching (URL, method, response body)

**5. Prompt drift invalidates recordings**
Recorded responses are specific to the conversation flow. If prime text,
hooks, or phase logic changes, the replayed responses won't match the
new tool calls. Recordings need to be re-recorded after prompt changes.
This is inherent to the approach — not solvable, only manageable.
