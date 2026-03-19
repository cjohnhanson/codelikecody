---
title: "Missouri services primitive for transitions and assertions"
status: in_progress
priority:
assignee:
labels: [missouri, feature]
depends_on: []
created: 2026-03-18T02:11:51Z
updated: "2026-03-18T03:09:33Z"
---

Add a `services:` key to TransitionConfig and AssertionConfig. Services are
background processes (e.g., an API server) that missouri starts before executing
the transition command or assertion, then stops afterward.

## Lifecycle

1. Missouri copies state to temp dir
2. Starts services defined on the transition
3. Runs the transition command (can talk to services)
4. Stops services
5. Compares filesystem against target state (services are down — no races)
6. Starts services defined on target state assertions
7. Runs assertions (can talk to services)
8. Stops services

## Config

```yaml
# on a transition
transitions:
  - name: "create issue via API"
    services: &api
      - command: "clc-api serve --port 0"
        ready: "curl -sf http://localhost:$PORT/health"
    command: "curl -X POST http://localhost:$PORT/api/issues ..."
    target: "../issue-created"

# on the target state's assertions (YAML anchor reuse)
assertions:
  - name: "API returns new issue"
    services: *api
    command: "curl -sf http://localhost:$PORT/api/issues | jq '.[0].title'"
    stdout: "\"my new issue\"\n"
```

## Port management

Services bind to port 0. The OS assigns an ephemeral port. The service
prints a line to stderr matching a pattern (default: `listening.*:(\d+)`).
Missouri captures the port and injects `$PORT` into the service's ready
command and the transition/assertion command's environment. Each concurrent
test path gets its own port — no collisions.

Same pattern as the existing mitmproxy integration (`parse_mitmdump_port`,
`MitmdumpHandle`), generalized.

## Config types

```rust
// In config.rs
struct ServiceConfig {
    command: String,
    ready: Option<String>,        // readiness check command
    shell: bool,                  // default true
    port_pattern: Option<String>, // regex, default "listening.*:(\d+)"
}

// Added to TransitionConfig and AssertionConfig
services: Vec<ServiceConfig>  // default empty
```

## Implementation

- `ServiceHandle` — RAII struct (like MitmdumpHandle): spawns process,
  parses port from stderr, kills on drop
- `start_service()` — spawn, wait for port, run ready check, return handle
- `execute_transition` starts services before command, drops handles after
- `run_single_assertion` starts services before assertion, drops after
- Port injected as `$PORT` env var (or `$PORT_0`, `$PORT_1` for multiple)

## Concurrency

Test paths run in parallel via rayon. Each path gets its own temp dir and
its own service instances with OS-assigned ports. No shared state between
paths.

## Scratch Notes

### 2026-03-18 — Implementation complete

All layers done:
- config.rs: ServiceConfig struct, services on TransitionConfig + AssertionConfig (6 tests)
- graph.rs: services on Transition + Assertion, wired into discover (3 tests)
- executor.rs: ServiceHandle (RAII, process group, SIGTERM/SIGKILL, stderr drain),
  start_service, build_service_env, run_ready_check, start_services.
  Integrated into execute_transition and run_single_assertion. (7 tests)
- illinois.rs: fixture 21-services, integration test passes
- Dependency: regex = "1" added to Cargo.toml

Total: 16 new tests, all passing. Pre-existing meltano nix failure unrelated.

Decisions made:
- No external crate for process groups — std process_group(0) + libc::kill(-pgid, sig)
- regex crate for configurable port_pattern (already transitive dep via termimad)
- Default port pattern: `listening.*:(\d+)`
- SIGTERM → 100ms → SIGKILL on drop
- Stderr drain thread prevents pipe buffer deadlock
