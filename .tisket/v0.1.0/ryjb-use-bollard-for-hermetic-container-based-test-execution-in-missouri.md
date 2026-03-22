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
