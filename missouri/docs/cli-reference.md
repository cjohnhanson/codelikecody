<!-- metadata
title: "missouri CLI Reference"
description: "Complete command reference for the missouri test framework"
type: reference
-->

# missouri CLI Reference

```
missouri [OPTIONS] <COMMAND>
```

End-to-end testing as directed graphs of filesystem states.

## Global Options

| Flag | Description |
|------|-------------|
| `-C <DIR>` | Change to this directory before doing anything. |
| `--config-dir <NAME>` | Name of the config directory. Default: `.missouri`. |
| `--version` | Print version. |
| `-h, --help` | Print help. |

## Commands

### `missouri run`

Run all test paths discovered in the state graph.

```
missouri run [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |
| `-v, --verbose` | Increase verbosity. Stackable: `-v`, `-vv`, `-vvv`. |
| `-q, --quiet` | Suppress non-essential output. |
| `--keep-temp` | Keep temp directories after the run (for debugging). |
| `--check-only` | Run only state assertions. Skip transitions and filesystem comparison. Conflicts with `--no-check` and `--record`. |
| `--no-check` | Skip all assertions. Run only transitions and filesystem comparison. Conflicts with `--check-only`. |
| `--record` | Record transition output to asciicast (`.cast`) files. Conflicts with `--check-only`. |
| `--run-id <ID>` | Custom run ID for the recording output directory. Default: timestamp (`YYYY-MM-DDTHH-MM-SS`). Requires `--record`. |

Exit codes: `0` all paths passed, `1` one or more failed, `2` configuration error, `130` interrupted.

### `missouri list`

List states, transitions, or test paths in the state graph.

```
missouri list [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |
| `--show <KIND>` | What to list. Default: `paths`. |

`--show` accepts:

| Value | Description |
|-------|-------------|
| `states` | Print all discovered states. |
| `transitions` | Print all discovered transitions. |
| `paths` | Print all enumerated test paths (root-to-leaf walks). |
| `graph` | Same as `paths`. |

### `missouri validate`

Validate `missouri.yml` files without running anything. Checks that configs parse, all transition targets resolve to real states, and at least one root state exists.

```
missouri validate [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |

Prints a summary: `valid: N state(s), N transition(s), N root(s)`.

### `missouri init`

Initialize a new missouri project by creating the config directory structure.

```
missouri init [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory for the project. Default: `.` |

### `missouri state add`

Add a new state to the project.

```
missouri state add <NAME> [OPTIONS]
```

| Argument / Flag | Description |
|-----------------|-------------|
| `<NAME>` | Name of the new state (becomes the directory name). |
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |
| `--from <STATE>` | Copy from an existing state and create a placeholder transition from it to the new state. |

### `missouri report`

Generate a report from recorded runs.

```
missouri report [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |
| `--format <FMT>` | Report format. Default: `terminal`. |
| `--run <ID>` | Specific run ID to report on. Default: latest. |

`--format` accepts:

| Value | Output |
|-------|--------|
| `terminal` | Print to stdout. |
| `html` | Write `report.html` to the run directory. |
| `md` | Write `report.md` to the run directory. |

### `missouri serve`

Serve an HTML report locally.

```
missouri serve [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-d, --dir <DIR>` | Root directory containing states. Default: `.` |
| `--run <ID>` | Specific run ID to serve. Default: latest. |
| `--port <PORT>` | Port to serve on. Default: `8080`. |

---

## Configuration Reference

Missouri uses YAML config files named `missouri.yml`. There are two levels: project-level (one per project root) and state-level (one per state directory).

### Config file locations

Project-level config is loaded from the first file found (in order):

1. `<root>/missouri.yml` -- root-level config (may include `test_dir`)
2. `<root>/<config_dir>/missouri.yml` -- config-dir-level config

If both exist, the root-level file wins.

State-level config lives at `<state_dir>/<config_dir>/missouri.yml`.

### Project-level `missouri.yml`

```yaml
# Directory containing test states, relative to this config file.
# When set, state discovery starts here instead of the project root.
test_dir: tests/smoke

# Environment variables inherited by all states.
# State-level env overrides these on collision.
env:
  RUST_BACKTRACE: "1"
  APP_ENV: test

# Commands run sequentially before any test paths.
# Execution stops on first failure.
setup:
  - name: "build project"        # optional label
    command: "cargo build --release"
    shell: true                   # default: true
  - command: "db-seed"
    shell: false

# Nix packages to make available via `nix shell`.
# When non-empty, all commands run inside a nix shell with these packages.
packages:
  - python3
  - uv
  - git

# Workspace mode: list of member directories.
# When set, `missouri run` iterates each member independently.
members:
  - clc/tests/missouri
  - tisket/tests/missouri
```

#### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `test_dir` | string | (none) | Redirect state discovery to a subdirectory. |
| `env` | map<string, string> | `{}` | Project-level environment variables. |
| `setup` | list of [SetupCommand](#setupcommand) | `[]` | Commands to run before test paths. |
| `packages` | list of string | `[]` | Nixpkgs packages for the sandbox. |
| `members` | list of string | `[]` | Workspace member directories. |

#### SetupCommand

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | auto-generated (`setup[N]`) | Human-readable label. |
| `command` | string | **required** | Command to execute. |
| `shell` | bool | `true` | Whether to run via `sh -c`. When false, command is split on whitespace and executed directly. |

### State-level `missouri.yml`

```yaml
# Environment variables for this state.
# Merged on top of project-level env (state wins on collision).
env:
  DB_URL: "postgres://localhost/test"

# Transitions out of this state.
transitions:
  - name: "build"                 # optional label
    command: "make build"
    target: "../built"            # relative path to target state directory
    shell: true                   # default: true
    stdout: "expected stdout\n"   # optional exact-match assertion
    stderr: ""                    # optional exact-match assertion

    # Background services (see Services section)
    services:
      - command: "my-server --port 0"

    # Network interception (see Network section)
    network:
      replay: recordings/worker.flow

    # Comparison overrides (see Comparators section)
    comparators:
      files:
        - path: "dist/manifest.json"
          command: "compare-json"
        - path: "logs/"
          ignore: true
      env:
        - name: BUILD_TIMESTAMP
          ignore: true
      network:
        - path: "api.example.com/v1/*"
          command: "compare-api"
        - path: "*.googleapis.com/**"
          ignore: true

# Assertions to verify properties of this state.
assertions:
  - name: "check output"         # optional label
    command: "echo hello"
    shell: true                   # default: true
    stdout: "hello\n"            # optional exact-match
    stderr: ""                   # optional exact-match
    should_fail: false           # default: false
    services:                    # optional background services
      - command: "my-server --port 0"
```

An empty config (`{}`) is valid -- this represents a terminal state with no outgoing transitions and no assertions.

#### Transition

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | auto-generated (`statename[N]`) | Human-readable label. |
| `command` | string | **required** | Command to execute. |
| `shell` | bool | `true` | Run via `sh -c`. When false, command is split on whitespace. |
| `target` | string | **required** | Relative path to the target state directory. |
| `stdout` | string | (none) | Expected stdout (exact match). Only checked in Full mode. |
| `stderr` | string | (none) | Expected stderr (exact match). Only checked in Full mode. |
| `services` | list of [Service](#service) | `[]` | Background services to run during this transition. |
| `network` | [NetworkConfig](#network-interception) | (none) | Network interception config. |
| `comparators` | [Comparators](#comparators) | (none) | Override how specific files, env vars, or network requests are compared. |

#### Assertion

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | auto-generated (`statename:assert[N]`) | Human-readable label. |
| `command` | string | **required** | Command to execute. |
| `shell` | bool | `true` | Run via `sh -c`. |
| `stdout` | string | (none) | Expected stdout (exact match). |
| `stderr` | string | (none) | Expected stderr (exact match). |
| `should_fail` | bool | `false` | When true, the assertion passes if the command exits non-zero. Stdout/stderr matching is skipped in this mode. |
| `services` | list of [Service](#service) | `[]` | Background services to run during this assertion. |

### Comparators

Override how specific paths, environment variables, or network requests are compared during a transition. Defined under `comparators` on a transition.

#### File comparators (`comparators.files`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | **required** | Relative path. Trailing `/` means directory subtree. |
| `command` | string | (none) | Custom comparator command. Receives two paths (actual, expected) as arguments. |
| `ignore` | bool | `false` | Exclude this path from comparison entirely. |

Exactly one of `command` or `ignore: true` should be specified.

#### Env comparators (`comparators.env`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | **required** | Environment variable name. |
| `command` | string | (none) | Custom comparator command. Receives two values as arguments. |
| `ignore` | bool | `false` | Exclude this env var from comparison. |

#### Network comparators (`comparators.network`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | **required** | URL path pattern (e.g. `"api.example.com/v1/messages"` or `"*.googleapis.com/**"`). |
| `command` | string | (none) | Custom comparator command. |
| `ignore` | bool | `false` | Exclude matching requests from comparison. |

### Network Interception

Configured per-transition under the `network` key. Uses mitmdump (from mitmproxy) for transparent HTTP/HTTPS interception. Exactly one mode per transition.

**Replay mode** -- replay previously recorded traffic:

```yaml
network:
  replay: recordings/worker.flow
```

The `replay` path is resolved relative to the source state's `<config_dir>/` directory.

**Record mode** -- capture traffic during the transition:

```yaml
network:
  record: true
```

Recorded flows are written to `<source_state>/<config_dir>/recordings/<transition_name>.flow`.

When network interception is active, the following environment variables are injected into the transition command:

| Variable | Value |
|----------|-------|
| `HTTPS_PROXY` | `http://127.0.0.1:<port>` |
| `HTTP_PROXY` | `http://127.0.0.1:<port>` |
| `NODE_EXTRA_CA_CERTS` | `~/.mitmproxy/mitmproxy-ca-cert.pem` |

mitmdump must be on PATH (add `mitmproxy` to `packages` or install manually).

### Services

Background services can be attached to transitions or assertions. A service is a long-running process that gets started before the command runs and killed (SIGTERM, then SIGKILL) after it finishes.

```yaml
services:
  - command: "my-server --port 0"
    shell: true                              # default: true
    port_pattern: "Serving on port (\\d+)"   # regex with one capture group
    ready: "curl -sf http://localhost:$PORT/health"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | **required** | Command to start the service. |
| `shell` | bool | `true` | Run via `sh -c`. |
| `port_pattern` | string | `listening.*:(\d+)` (runtime default) | Regex to extract the port number from stderr. Must contain exactly one capture group. The default is applied at runtime, not in the config schema — omitting this field uses the default pattern. |
| `ready` | string | (none) | Readiness check command. Retried with exponential backoff (100ms to 5s, up to 10 attempts). `$PORT` is available in the environment. |

**Port injection:** After a service starts and its port is captured from stderr, the port is made available via environment variables:

- Single service: `$PORT`
- Multiple services: `$PORT` (first service), `$PORT_0`, `$PORT_1`, etc.

Services are spawned in their own process group so the entire process tree can be killed on cleanup. A 30-second timeout applies to port detection -- if the service doesn't print a matching line to stderr within that window, it's killed and the step fails.

### Ignore Patterns

The file `<config_dir>/ignore` (e.g., `.missouri/ignore`) uses gitignore syntax to exclude paths from filesystem comparison across all transitions.

```
# .missouri/ignore
*.log
tmp/
!important.log
```

Standard gitignore rules apply: trailing `/` matches directories, `!` negates a pattern, `**` matches across directory boundaries, `#` starts a comment.

When `test_dir` is set, the ignore file is loaded from the test directory's config dir (e.g., `tests/.missouri/ignore`).

### Shared Bin Directory

Executables placed in `<config_dir>/bin/` are prepended to PATH for all commands. This works at two levels:

- **Project-level:** `<root>/<config_dir>/bin/` -- available to all states and transitions.
- **State-level:** `<state>/<config_dir>/bin/` -- available to transitions originating from that state and its assertions.

PATH resolution order: state bin -> project bin -> base PATH.

When `test_dir` is set, the project bin is looked up in the test directory first, then falls back to the root config directory.

### Sandbox / Packages

When `packages` is set in the project config, all commands run inside `nix shell nixpkgs#pkg1 nixpkgs#pkg2 ... --command`. Missouri resolves the nixpkgs flake reference to a pinned commit hash during a warm-up phase (before parallel execution begins) to avoid registry file contention.

The `MISSOURI_SANDBOX` environment variable overrides sandbox behavior:

| Value | Effect |
|-------|--------|
| `preinstalled` | Skip nix shell entirely. Assume all packages are already on PATH. Useful inside nix derivations where packages are `nativeCheckInputs`. |

If `packages` is non-empty and `nix` is not found on PATH (and `MISSOURI_SANDBOX` is not `preinstalled`), missouri exits with an error.

---

## State Graph Model

Missouri models tests as a directed graph where **states** are nodes and **transitions** are edges.

**State:** A directory on disk. Its contents represent a snapshot of the filesystem at a particular point. Each state directory contains a `<config_dir>/missouri.yml` describing its outgoing transitions and assertions.

**Transition:** A command that transforms the filesystem from one state to another. After running the command in a temp copy of the source state, missouri diffs the result against the expected target state.

**Root state:** A state with no inbound transitions. Test path enumeration starts from roots.

**Terminal state:** A state with no outgoing transitions (an empty config or one with only assertions). Test paths end here.

**Test path:** A root-to-terminal walk through the graph. Missouri enumerates all such paths and runs them in parallel. For a graph with branching (`A -> B`, `A -> C`), this produces two paths.

**Chained paths:** For multi-step paths (`A -> B -> C`), the temp directory from one transition is carried forward as the input for the next -- the intermediate state is not re-copied from disk.

**Assertions** run at state boundaries. In Full mode (default), source state assertions run before the first transition, and target state assertions run after each transition. In CheckOnly mode, only assertions run (no transitions, no filesystem comparison). In NoCheck mode, assertions are skipped entirely.

**Workspace mode:** When `members` is set in the project config, missouri treats each member directory as an independent project and runs them sequentially, reporting results per member.
