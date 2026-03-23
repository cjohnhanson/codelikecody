---
title: "Network mocking with mitmdump inside microsandbox"
status: cancelled
priority: 2
assignee:
labels: [missouri, network, microsandbox]
depends_on: [6851-missouri-microsandbox-backend-for-hermetic-test-execution]
created: 2026-03-20T02:57:41Z
updated: "2026-03-23T00:08:51Z"
---

mitmdump record/replay runs inside microsandbox alongside the transition
command. `NetworkScope::None` on the sandbox means no traffic leaks —
the macOS per-process-tree scoping problem that blocked the original
mitmproxy approach doesn't exist.

## Prior work

The `mitmproxy-network-mocking` branch has significant implementation:
- `NetworkConfig` enum (Replay/Record) with serde
- `NetworkComparatorConfig` for traffic comparison
- `build_network_env(port)` — HTTPS_PROXY/HTTP_PROXY/NODE_EXTRA_CA_CERTS
- `start_mitmdump_replay(flow, path_env)` / `start_mitmdump_record(output, path_env)`
- `MitmdumpHandle` RAII struct with port parsing from stderr
- `execute_transition` integration for both replay and record modes
- 79 unit tests, all passing

See `mitmproxy-network-mocking` tisket scratch notes (sessions 2026-03-16
and 2026-03-17) for full implementation details.

## What changes

The executor helpers (`start_mitmdump_replay`, `MitmdumpHandle`, etc.) move
inside the microsandbox backend. Instead of spawning mitmdump on the host
and injecting HTTPS_PROXY into the command env, missouri:

1. Boots the test sandbox (which has mitmdump in the image via flake.nix)
2. Starts mitmdump inside the sandbox (replay or record mode)
3. Injects HTTPS_PROXY into the transition command's env inside the sandbox
4. Runs the transition command
5. Tears down mitmdump

The `network:` config key on transitions stays the same. Flow files
live in the state directory. The transition-level API is unchanged.

## What this subsumes

- `mitmproxy-network-mocking` tisket (fully)
- `mitmdump-snapshotting-tooling` tisket
- `composable-sandbox-model` tisket (microsandbox provides all isolation layers)
