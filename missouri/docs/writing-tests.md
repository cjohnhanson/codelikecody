<!-- metadata
title: "Writing Missouri Tests"
description: "How to model tests as state graphs with transitions, assertions, and services"
type: guide
-->

# Writing Missouri Tests

## The state graph model

A missouri test suite is a directed graph of filesystem states connected by
transitions. Each state is a directory containing the files you'd expect to
exist at that point. Each transition is a shell command that transforms one
state into another. Missouri copies the source state to a temp directory,
runs the command, then diffs the result against the expected target state.

The mental model: if your CLI starts with files A and you run
`my-tool init`, the result should look like files B. State A has a
transition `my-tool init` targeting state B. Missouri verifies that's true.

States can also carry assertions -- commands that verify properties not
captured by the filesystem snapshot (exit codes, stdout content, tool
behavior).

## Directory structure

```
my-project/tests/missouri/
├── .missouri/
│   ├── missouri.yml      # project-level config
│   ├── ignore            # gitignore-syntax patterns to exclude from comparison
│   └── bin/              # scripts on PATH during test runs
├── state-a/
│   ├── .missouri/
│   │   └── missouri.yml  # state config (transitions, assertions)
│   ├── file.txt          # fixture files for this state
│   └── src/
│       └── main.rs
├── state-b/
│   ├── .missouri/
│   │   └── missouri.yml
│   └── ...
```

Each state directory contains:

1. A `.missouri/missouri.yml` config file (transitions, assertions, env)
2. The actual files that represent this state (the fixture)

Missouri discovers states by walking the tree looking for
`<config_dir>/missouri.yml` files. The project root's config is
project-level, not a state.

## Project-level config

The project-level `missouri.yml` lives either at the test suite root
(`tests/missouri/.missouri/missouri.yml`) or as a root-level file
(`tests/missouri/missouri.yml`). Root-level takes precedence if both exist.

```yaml
# Project-level environment variables (inherited by all states)
env:
  NO_COLOR: "1"

# Setup commands run once before any test paths
setup:
  - name: "build project"
    command: "cargo build --quiet --manifest-path ../../Cargo.toml"

# Nix packages to make available during test runs
packages:
  - git
  - jq

# Optional: point state discovery at a subdirectory
test_dir: tests/smoke

# Optional: workspace mode -- iterate multiple member suites
members:
  - clc/tests/missouri
  - tisket/tests/missouri
```

> **Missouri clears the environment.** Commands run with only `PATH` and whatever is declared in `env` blocks — nothing else. No `HOME`, no `TMPDIR`, no `SHELL`. If a command fails mysteriously, check whether it depends on a variable that wasn't declared.

### Field reference (ProjectConfig)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `env` | map | `{}` | Environment variables inherited by all states |
| `setup` | list | `[]` | Commands to run before any test paths |
| `packages` | list | `[]` | Nix packages to provide via `nix shell` |
| `test_dir` | string | none | Directory for state discovery (relative to config) |
| `members` | list | `[]` | Workspace member directories |
| `microsandbox` | bool | `false` | Run transitions inside microsandbox microVMs |

### Setup commands

Setup commands run sequentially before any test paths. They execute on the
host (never inside a sandbox), from the project root directory. Common use:
building the binary under test.

```yaml
setup:
  - name: "build tisket"
    command: "cargo build --quiet --manifest-path ../../Cargo.toml"
  - command: "db-seed"
    shell: false
```

Each setup command has `command` (required), optional `name`, and optional
`shell` (defaults to `true` -- runs via `sh -c`). Set `shell: false` to
exec the command directly.

### The ignore file

Place an `ignore` file at `.missouri/ignore` (gitignore syntax). Patterns
listed here are excluded from filesystem comparison across all transitions.

```
# .missouri/ignore
.git/
```

The clc test suite ignores `.git/` because git internals are
non-deterministic. The comparison engine uses the `ignore` crate, so full
gitignore syntax works: `*`, `**`, `!` negation, `#` comments, trailing
`/` for directories.

### The bin directory

Scripts in `.missouri/bin/` are prepended to PATH during test execution.
This is the right place for custom comparators, test helpers, and wrapper
scripts.

```
.missouri/bin/
├── validate-settings    # custom comparator
├── compare-issue        # another custom comparator
└── setup-divergent-branch  # test helper
```

## Writing states

### Fixture files

A state directory contains the files you expect to exist at that point in
the test. When missouri runs a transition, it copies the source state to a
temp dir, executes the command, then diffs the temp dir against the target
state directory.

Files in `.missouri/` are never part of the fixture -- they're config.

### Dotfile fixtures via dot- directories

Git can't track directories like `.git/` or `.clc/` inside your test
fixtures. Missouri solves this with the `dot-` convention: a directory
named `.missouri/dot-<name>/` gets restored as `.<name>/` in the temp dir
at runtime.

```
initialized/.missouri/
├── missouri.yml
├── dot-git/         # becomes .git/ at runtime
│   ├── HEAD
│   └── config
└── dot-clc/         # becomes .clc/ at runtime
    └── .gitkeep     # .gitkeep files are skipped during restoration
```

`.gitkeep` files inside `dot-` directories are automatically skipped --
they exist only to make git track the otherwise-empty directory.

### Entrypoints

By default, missouri traces paths starting from root states (states with no
inbound transitions). To mark a state as a valid starting point for a
subgraph, set `entrypoint: true`:

```yaml
entrypoint: true

assertions:
  - name: "everything looks right"
    command: "test -d .clc"
```

This is useful when a state is expensive to reach via transitions and you
want to test from a pre-built snapshot.

### Environment variables

States inherit project-level env and can override or add their own:

```yaml
env:
  APP_ENV: test
  DB_URL: "postgres://localhost/test"
```

Merge order: project env is the base, state env overrides. The environment
is cleared before execution (`env_clear`), so only explicitly declared
variables and `PATH` are available.

## Writing transitions

A transition connects two states: "run this command on the source state and
the result should match the target state."

```yaml
transitions:
  - name: "initialize project"
    command: "my-tool init"
    target: "../initialized"
```

### Field reference (TransitionConfig)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | auto-generated | Human-readable label |
| `command` | string | **required** | Shell command to execute |
| `target` | path | **required** | Relative path to target state directory |
| `shell` | bool | `true` | Run via `sh -c` |
| `comparators` | object | none | Override comparison for specific files/env/network |
| `network` | object | none | Network interception config (replay/record) |
| `stdout` | string | none | Expected stdout (exact match) |
| `stderr` | string | none | Expected stderr (exact match) |
| `services` | list | `[]` | Background services to run during this transition |

### Target resolution

Targets are relative paths resolved from the source state directory. The
typical pattern is `../sibling-state`:

```
states/
├── before/          # source
│   └── .missouri/missouri.yml  →  target: "../after"
└── after/           # target
```

Deeper nesting works too: `../../other-suite/some-state`. The path must
resolve to a directory that contains a `.missouri/missouri.yml`.

### Multi-step transitions

A state can have multiple outgoing transitions, and a target state can have
its own transitions. Missouri discovers all paths through the graph and
tests each one. For chained paths (A -> B -> C), the output of
transition A->B becomes the input for transition B->C.

```yaml
# state-a/missouri.yml
transitions:
  - name: "step one"
    command: "init-tool"
    target: "../state-b"

# state-b/missouri.yml
transitions:
  - name: "step two"
    command: "run-tool"
    target: "../state-c"
```

Missouri will discover the path `state-a -> state-b -> state-c` and run
both transitions in sequence.

### Branching

A single state can have multiple transitions to different targets, modeling
different outcomes:

```yaml
transitions:
  - name: "close issue"
    command: "tisket issue close fix-the-widget"
    target: "../issue-closed"
  - name: "edit issue"
    command: "tisket issue edit fix-the-widget --status todo"
    target: "../issue-edited"
  - name: "create second issue"
    command: "tisket issue create 'Write tests' -p bugs"
    target: "../has-two-issues"
```

Each branch becomes a separate test path.

### Shell vs direct execution

By default, commands run via `sh -c`, so pipes, redirects, and
multi-statement commands work:

```yaml
command: "git init -q -b main && my-tool init"
```

Set `shell: false` for direct execution (no shell interpretation):

```yaml
command: "/usr/bin/my-tool"
shell: false
```

### Stdout and stderr assertions on transitions

To assert exact command output alongside the filesystem diff:

```yaml
transitions:
  - name: "echo test"
    command: "echo hello"
    target: "../next"
    stdout: "hello\n"
    stderr: ""
```

These are exact-match comparisons. When omitted, output is not checked.

## Writing assertions

Assertions are commands attached to a state that verify properties not
captured by the filesystem snapshot. They run against the state's fixture
(copied to a temp dir).

```yaml
assertions:
  - name: "config file exists"
    command: "test -f .clc/config.yml"

  - name: "config show reflects custom value"
    command: "clc config show 2>&1 | grep 'main_branch: trunk'"

  - name: "issue list is empty"
    command: "tisket issue list"
    stdout: ""
```

### Field reference (AssertionConfig)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | auto-generated | Human-readable label |
| `command` | string | **required** | Command to execute |
| `shell` | bool | `true` | Run via `sh -c` |
| `stdout` | string | none | Expected stdout (exact match) |
| `stderr` | string | none | Expected stderr (exact match) |
| `should_fail` | bool | `false` | Pass when the command exits non-zero |
| `services` | list | `[]` | Background services to run during this assertion |

### When to use assertions vs transitions

Use **transitions** when you're testing a state transformation: "command X
on state A produces state B." The filesystem diff is the primary check.

Use **assertions** when you're testing properties of a state in place:
command exit codes, stdout content, behavior that depends on runtime state
(like git branches).

A state can have both transitions and assertions. Assertions run against the
state fixture; transitions run against it and then diff the result.

### Expecting failure

To assert that a command *should* fail:

```yaml
assertions:
  - name: "init when already initialized fails"
    command: "tisket init"
    should_fail: true
    stderr: "error: already initialized (tisket.yml exists)\n"
```

When `should_fail: true`, the assertion passes if the command exits
non-zero. Combine with `stderr` to verify the error message.

### States with only assertions (no transitions)

Terminal states (no outgoing transitions) commonly carry only assertions.
They verify the result of the transition that led there:

```yaml
# issue-closed/.missouri/missouri.yml
assertions:
  - name: "issue status is done"
    command: "grep -q 'status: done' .tisket/default/fix-the-widget.md"
```

Root states can also be assertion-only. Combined with `entrypoint: true`,
these are useful for verifying a pre-built snapshot:

```yaml
entrypoint: true

assertions:
  - name: ".clc directory exists"
    command: "test -d .clc"
  - name: "settings.local.json is valid JSON"
    command: "jq empty .claude/settings.local.json"
```

## Custom comparators

By default, missouri does a recursive file-by-file diff between the actual
output and the expected target state. To override this for specific paths:

```yaml
transitions:
  - command: "clc init"
    target: "../initialized"
    comparators:
      files:
        - path: ".claude/settings.local.json"
          command: "validate-settings"
        - path: "logs/"
          ignore: true
        - path: ".git/"
          ignore: true
```

### File comparators

Each entry under `comparators.files` has:

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Relative path. Trailing `/` matches a directory subtree |
| `command` | string | Custom comparator command (receives actual and expected as args) |
| `ignore` | bool | Exclude this path from comparison entirely |

A custom comparator command receives two arguments: the actual file path
and the expected file path. Exit 0 to pass, non-zero to fail.

```bash
#!/usr/bin/env bash
# .missouri/bin/validate-settings
# $1 = actual file, $2 = expected file
set -euo pipefail
jq empty "$1" || { echo "FAIL: not valid JSON"; exit 1; }
jq -e '.hooks' "$1" >/dev/null || { echo "FAIL: missing hooks"; exit 1; }
```

### Ignoring paths

The `ignore: true` flag is for paths that change non-deterministically or
aren't part of what you're testing:

```yaml
comparators:
  files:
    - path: ".clc/"
      ignore: true
    - path: ".git/"
      ignore: true
    - path: ".worktrees/"
      ignore: true
```

This is per-transition. For project-wide ignores, use the `.missouri/ignore`
file instead.

### Environment variable comparators

Override comparison for specific environment variables:

```yaml
comparators:
  env:
    - name: BUILD_TIMESTAMP
      ignore: true
    - name: VERSION
      command: "compare-semver"
```

### Network request comparators

When using network interception, override comparison for specific request
patterns:

```yaml
comparators:
  network:
    - path: "api.anthropic.com/v1/messages"
      command: "compare-api-calls"
    - path: "*.googleapis.com/**"
      ignore: true
```

## Background services

Transitions and assertions can start background services (servers, daemons)
that are automatically started before the command runs and killed afterward.

```yaml
transitions:
  - command: "curl http://localhost:$PORT/"
    target: "../next"
    services:
      - command: "my-server --port 0"
```

Missouri watches the service's stderr for a port announcement (default
pattern: `listening.*:(\d+)`), captures the port, and exposes it as `$PORT`
in the transition/assertion command's environment.

### Field reference (ServiceConfig)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | **required** | Command to start the service |
| `shell` | bool | `true` | Run via `sh -c` |
| `port_pattern` | string | `listening.*:(\d+)` | Regex to extract port from stderr (one capture group) |
| `ready` | string | none | Readiness check command. `$PORT` is available. Retried with backoff |

### Readiness checks

If the service needs time to become ready, use `ready`:

```yaml
services:
  - command: "/usr/bin/my-server"
    shell: false
    port_pattern: "Serving on port (\\d+)"
    ready: "curl -sf http://localhost:$PORT/health"
```

The readiness check retries up to 10 times with exponential backoff
(starting at 100ms, capped at 5s).

### Multiple services

With multiple services, ports are exposed as `$PORT_0`, `$PORT_1`, etc.
`$PORT` is always set to the first service's port.

```yaml
services:
  - command: "server-a --port 0"
  - command: "server-b --port 0"
    ready: "curl -sf http://localhost:$PORT_1/ready"
```

### Services on assertions

Works the same way:

```yaml
assertions:
  - command: "curl -sf http://localhost:$PORT/"
    services:
      - command: "my-server --port 0"
```

## Network interception

Transitions can intercept HTTP/HTTPS traffic via mitmproxy for
record/replay testing.

### Replay mode

Replay previously recorded traffic:

```yaml
transitions:
  - command: "clc dispatch test"
    target: "../next"
    network:
      replay: .missouri/recordings/worker.flow
```

The `replay` path is relative to the source state directory.

### Record mode

Record traffic during a transition:

```yaml
transitions:
  - command: "clc dispatch test"
    target: "../next"
    network:
      record: true
```

When recording, missouri starts mitmdump, sets `HTTPS_PROXY`, `HTTP_PROXY`,
and `NODE_EXTRA_CA_CERTS` in the command's environment, and saves the
captured flow file.

## Running tests

```bash
# Run all test paths
missouri run -d tests/missouri

# Verbose output (show passing steps too)
missouri run -d tests/missouri -v

# Keep temp directories for debugging
missouri run -d tests/missouri --keep-temp

# Run only assertions (skip transitions and filesystem comparison)
missouri run -d tests/missouri --check-only

# Run only transitions and filesystem comparison (skip assertions)
missouri run -d tests/missouri --no-check

# Record transition output
missouri run -d tests/missouri --record
```

### Check modes

| Flag | Transitions | Filesystem diff | Assertions |
|------|-------------|-----------------|------------|
| (none) | yes | yes | yes |
| `--check-only` | no | no | yes |
| `--no-check` | yes | yes | no |

`--check-only` is useful for fast iteration on assertions without
re-running transitions. `--no-check` is useful for updating fixtures
after a change -- run transitions, inspect the diff, update expected state.

### Debugging failures

**Verbose output** (`-v`): Shows passing steps, not just failures. All
assertion output, command stdout/stderr, and comparison details.

**Keep temp directories** (`--keep-temp`): After a run, the temp directories
where transitions executed are preserved instead of cleaned up. The paths
are printed in the output. Inspect them to see exactly what the command
produced.

**List paths** before running to understand the graph:

```bash
missouri list paths -d tests/missouri
missouri list states -d tests/missouri
missouri list transitions -d tests/missouri
```

## Patterns from the codebase

### Pattern: build-then-test with setup

Both clc and tisket test suites build the binary under test in the setup
phase, ensuring the PATH binary matches the current source:

```yaml
# .missouri/missouri.yml
setup:
  - name: "build clc"
    command: "cargo build --quiet --manifest-path ../../Cargo.toml"
packages:
  - git
  - jq
```

### Pattern: ignore non-deterministic paths per transition

When a transition creates or modifies files that aren't the point of the
test, ignore them:

```yaml
transitions:
  - name: "close issue"
    command: "tisket issue close fix-the-widget"
    target: "../issue-closed"
    comparators:
      files:
        - path: ".tisket/bugs/"
          ignore: true
```

### Pattern: assertion-heavy root states

The clc `initialized` state has dozens of assertions verifying the result
of `clc init` -- checking file existence, JSON structure, hook wiring,
command behavior. This catches regressions in the initialization path
without needing separate transitions for each check.

### Pattern: custom comparator scripts in bin/

For files with non-deterministic content (like JSON with embedded paths),
write a comparator script that validates structure rather than exact bytes:

```bash
#!/usr/bin/env bash
# .missouri/bin/validate-settings
# Missouri passes: $1 = actual file, $2 = expected file
set -euo pipefail
jq empty "$1" || { echo "FAIL: not valid JSON"; exit 1; }
jq -e '.hooks' "$1" >/dev/null || { echo "FAIL: missing hooks"; exit 1; }
```

Then reference it by name (it's on PATH):

```yaml
comparators:
  files:
    - path: ".claude/settings.local.json"
      command: "validate-settings"
```

### Pattern: multi-command transitions

Transitions can be complex multi-step shell commands when the setup is part
of the transition itself:

```yaml
transitions:
  - name: "setup git repo with branches"
    command: >
      git init -q -b main &&
      git -c user.name=test -c user.email=test@test add -A &&
      git -c user.name=test -c user.email=test@test commit -q -m "init" &&
      my-tool init &&
      git -c user.name=test -c user.email=test@test add -A &&
      git -c user.name=test -c user.email=test@test commit -q -m "tool init"
    target: "../ready-state"
    comparators:
      files:
        - path: ".git/"
          ignore: true
```

### Pattern: should_fail for error paths

Test that commands fail correctly with expected error messages:

```yaml
assertions:
  - name: "create issue in nonexistent project fails"
    command: "tisket issue create 'Something' -p nonexistent"
    should_fail: true
    stderr: "error: project 'nonexistent' not found\n"

  - name: "close nonexistent issue fails"
    command: "tisket issue close foo"
    should_fail: true
    stderr: "error: issue 'foo' not found\n"
```

## Further reading

- [What is Missouri?](/missouri/what-is-missouri) — the design philosophy behind filesystem state graphs
- [CLI Reference](/missouri/cli-reference) — full command and config schema reference
- [Getting Started](/missouri/getting-started) — build your first test suite from scratch
