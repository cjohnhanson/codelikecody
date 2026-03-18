---
title: "Hermetic network interception for missouri"
status: in_progress
priority:
assignee:
labels: [missouri, feature]
depends_on: []
created: 2026-02-23T00:00:00Z
updated: "2026-03-18T01:49:25Z"
---

Network is another dimension of sandbox control in missouri, alongside
filesystem (state directories) and environment (env_clear + explicit vars).
Today missouri controls what binaries are available (nix) and what env
the command sees. This adds control over what network the command sees.

Language-agnostic HTTP record/replay via mitmproxy. Missouri wraps
transition commands with HTTPS_PROXY pointed at mitmdump, so any
subprocess making HTTP calls gets intercepted regardless of language
or HTTP library. No application-level changes — the process under test
doesn't know it's being intercepted.

## Motivation

clc's autonomous workflow (dispatch → worker → land) is fully implemented
and tested via missouri, but worker tests can't exercise the full loop
because `clc dispatch` spawns a real claude process that talks to the
Anthropic API. Network replay would make hermetic full-loop tests possible:
record a real worker session once, replay it deterministically in missouri.

## Design

### Transition-level `network:` key

New key on `TransitionConfig`, sibling to `command`, `target`, `comparators`:

```yaml
# source state missouri.yml
transitions:
  - name: "worker completes the tisket"
    command: "clc dispatch test-feature"
    network:
      replay: recordings/worker.flow    # replay from this file
    target: "../worker-completed"
```

Recording mode (capture traffic during a transition):

```yaml
transitions:
  - name: "worker completes the tisket"
    command: "clc dispatch test-feature"
    network:
      record: true
    target: "../worker-completed"
```

Recording files live in the source state's `.missouri/` directory
(e.g., `.missouri/recordings/worker.flow`). They describe what happens
during the transition — they're an input to producing the target state,
not a property of it.

### mitmproxy is implicit

When `network:` is configured on a transition, missouri ensures mitmproxy
is available. Test authors don't declare it in `packages:`. Missouri
handles the dependency — either as a hard dep of the missouri binary
or baked into the nix expression the backend builds.

### Backend handles lifecycle

The backend (NixBackend or BareBackend) handles:

1. Start mitmdump in replay mode (or record mode) with the flow file
2. Wait for readiness (port binding)
3. Inject `HTTPS_PROXY` and `NODE_EXTRA_CA_CERTS` into the env it
   already controls (env_clear means no leaking)
4. Run the actual command
5. Tear down mitmdump
6. If recording, stash the flow file in `.missouri/recordings/`

The env injection is natural — the backend already owns environment
construction. HTTPS_PROXY is standard; NODE_EXTRA_CA_CERTS handles
TLS trust for the mitmproxy CA cert (~/.mitmproxy/mitmproxy-ca-cert.pem).

### Network traffic as a comparable output

Network traffic is an output of the transition, same as filesystem
changes and stdout/stderr. Missouri already compares filesystem trees
and command output between actual and expected. Network traffic
comparison follows the same pattern.

During a transition with `network: { record: true }`, mitmdump
captures traffic to a HAR file. The target state can have an expected
HAR in its `.missouri/` directory. Missouri compares them using the
same comparator override pattern as filesystem comparison:

```yaml
transitions:
  - name: "worker completes"
    command: "clc dispatch test-feature"
    network:
      record: true
    target: "../worker-completed"
    comparators:
      network:
        - path: "api.anthropic.com/v1/messages"
          command: "compare-api-calls"
        - path: "*.googleapis.com/**"
          ignore: true
```

This gives three things in one: hermetic replay for deterministic tests,
regression detection when API call patterns change, and documentation
of what network calls a transition makes.

Exact byte-for-byte matching on network traffic is wrong by default —
request headers have timestamps, auth tokens rotate, request IDs are
random. Custom comparators handle this, same as filesystem comparators.

## Implementation scope

### Config layer (config.rs, graph.rs) — DONE

- `NetworkConfig` enum (`Replay { replay }`, `Record { record }`) with `#[serde(untagged)]`
- `NetworkComparatorConfig` struct (`path`, `command`, `ignore`)
- `TransitionConfig.network: Option<NetworkConfig>`
- `ComparatorsConfig.network: Vec<NetworkComparatorConfig>`
- `graph::NetworkComparator` enum (`Ignore`, `Custom { command }`)
- `Transition.network` and `Transition.network_comparators` fields
- `resolve_network_comparators()` function
- 12 unit tests across config.rs, graph.rs, executor.rs

### Executor helpers — DONE

- `build_network_env(port)` — sets HTTPS_PROXY, HTTP_PROXY, NODE_EXTRA_CA_CERTS
- `start_mitmdump_replay(flow, path_env)` — finds mitmdump on PATH, spawns replay
- `MitmdumpHandle` — RAII struct that kills process on drop

### Executor integration (replay) — DONE

- `execute_transition` checks `transition.network` for `Replay`
- Starts mitmdump, parses port from stderr, merges proxy env into command
- `MitmdumpHandle` with `port` field, RAII cleanup
- 2 integration tests (fails-without-mitmdump, injects-proxy-env)

### Record mode — TODO

- `execute_transition` handles `Record` variant (currently no-op stub)
- Start mitmdump in record mode (`-w` flag) during transition
- Stash captured flow file in source state's `.missouri/recordings/`

### Build the actual missouri test — TODO

- Record a real `clc dispatch` session with mitmdump capturing API traffic
- Create missouri test path: dispatched-with-replay → worker-completed → merged
- This is the end goal: hermetic full-loop test of the autonomous workflow

### Comparison layer (compare.rs) — DEFERRED

- `NetworkDiff` variant alongside `FileDiff` and `EnvDiff`
- HAR comparison with comparator overrides
- Not needed for the core record/replay goal

### mitmproxy dependency — DEFERRED

- NixBackend auto-adding mitmproxy to packages
- Not blocking — can be declared in `packages:` manually for now

## What this subsumes

This tisket replaces:
- `mitmdump-snapshotting-tooling` — recording workflow is part of this
- The original thin `mitmproxy-network-mocking` stub

## Scratch Notes

### Session 2026-03-16

**Tests written (tests-unwritten → tests-written):**

Files consulted:
- `missouri/src/config.rs` — TransitionConfig, ComparatorsConfig, parse_config()
- `missouri/src/graph.rs` — Transition, FileComparator/EnvComparator pattern
- `missouri/src/executor.rs` — Backend trait, BareBackend, NixBackend, detect_sandbox
- `missouri/src/compare.rs` — FileDiff, EnvDiff patterns

No tests/missouri/ directory exists yet — all tests are unit tests inline in source files.

**Tests added:**
- `config.rs`: parse_network_config_replay, parse_network_config_record, parse_network_config_absent, parse_network_comparators, parse_network_comparators_absent
- `graph.rs`: discover_transition_network_replay, discover_transition_network_record, discover_transition_network_absent, discover_network_comparators_resolved
- `executor.rs`: build_network_env_sets_https_proxy, build_network_env_sets_ca_cert, start_mitmdump_replay_errors_when_not_on_path

**Types needed (don't exist yet):**
- `config::NetworkConfig` enum — `Replay { replay: Utf8PathBuf }` and `Record` variants
  - Use `#[serde(untagged)]` so `{replay: path}` and `{record: true}` both deserialize
- `config::NetworkComparatorConfig` — `{ path: String, command: Option<String>, ignore: bool }`
- `TransitionConfig.network: Option<NetworkConfig>`
- `ComparatorsConfig.network: Vec<NetworkComparatorConfig>` (default empty)
- `graph::NetworkComparator` enum — `Ignore` and `Custom { command }` (mirrors FileComparator)
- `Transition.network: Option<config::NetworkConfig>`
- `Transition.network_comparators: Vec<(String, NetworkComparator)>`
- `executor::build_network_env(port: u16) -> BTreeMap<String, String>`
  - Sets HTTPS_PROXY=http://127.0.0.1:{port}, HTTP_PROXY same, NODE_EXTRA_CA_CERTS=~/.mitmproxy/mitmproxy-ca-cert.pem
- `executor::start_mitmdump_replay(flow: &Utf8Path, path: &str) -> Result<MitmdumpHandle, String>`
  - Errors if mitmdump not found on given PATH

**Implementation plan (next phase):**
1. config.rs: Add NetworkConfig enum + NetworkComparatorConfig + wire into TransitionConfig/ComparatorsConfig
2. graph.rs: Add NetworkComparator + wire network/network_comparators into Transition resolve
3. executor.rs: Add build_network_env + start_mitmdump_replay + MitmdumpHandle (RAII for process teardown) + wire into run_transition

**Deferred (not in this scope):**
- NetworkDiff in compare.rs (HAR comparison) — complex, design doc says "same pattern as filesystem", but HAR parsing not needed to make these tests pass
- NixBackend auto-adding mitmproxy — lower priority, need tests first

### Session 2026-03-17

**Executor integration (tests-unwritten → green):**

Tests added to executor.rs:
- `execute_transition_network_replay_fails_without_mitmdump` — verifies step fails with mitmdump error when binary not on PATH
- `execute_transition_network_replay_injects_proxy_env` — verifies HTTPS_PROXY injected via fake mitmdump (python3 TCP listener script)

Implementation in executor.rs:
- `start_mitmdump_replay` now pipes stderr and parses port from "listening at http://*:PORT" line via `parse_mitmdump_port()`
- `MitmdumpHandle` now has `pub port: u16` field
- `execute_transition` checks `transition.network`:
  - `Replay { replay }`: resolves flow path relative to source state's config dir, starts mitmdump, merges network env (HTTPS_PROXY/HTTP_PROXY/NODE_EXTRA_CA_CERTS) into command env via `Cow<BTreeMap>`, early-returns failed StepResult if mitmdump errors
  - `Record`: TODO stub (no-op)
  - `None`: unchanged behavior

All 76 unit tests pass. 53 CLI tests pass. 17 illinois tests pass (except pre-existing meltano failure).
