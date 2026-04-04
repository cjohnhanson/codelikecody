<!-- metadata
title: "What is Missouri?"
description: "Why filesystem state graphs and how missouri's testing model works"
type: explanation
-->

# What is Missouri?

*Show-me-state: e2e testing as directed graphs of filesystem states.*

Missouri tests CLI tools by making one claim over and over: "after running
this command against this filesystem, the filesystem should look like that."
Every state is a directory on disk. Every transition is a shell command.
Every assertion is a diff.

## States and transitions

A test state is a directory containing the exact files that should exist
at that point. A transition is a command that runs against a copy of one
state and should produce another. Verification is a recursive diff between
what the command actually produced and what the target state directory
contains. Extra files, missing files, content mismatches are all failures.

There's no assertion DSL. The expected state *is the directory*.

## Why directed graphs

A linear test (state A → state B → state C) works for simple sequences, but
CLI tools don't have linear behavior. From one starting state, multiple
commands might apply. After running one command, several different follow-up
commands might be worth testing. The result is a directed graph.

Missouri models this explicitly. Each state directory contains a
`.missouri/missouri.yml` that declares transitions — the command to run and
which state directory the result should match. A state can have multiple
outgoing transitions (branching), and multiple states can transition into the
same target (convergence).

If 30 test paths share the same initial state, that state is defined
once. Adding a new test scenario means adding a directory and a
transition; existing states don't change. The directory tree is the test
suite. Walking it shows every state the tool under test can produce.

## State discovery

Missouri discovers the graph by walking the filesystem. Any subdirectory
containing `.missouri/missouri.yml` is a state. The project root's
`missouri.yml` (or `.missouri/missouri.yml`) is project-level config — env
vars, setup commands, sandbox settings — not a state.

Discovery is recursive. States can be nested at any depth below the root.
Hidden directories (starting with `.`) are skipped during discovery. The
graph is built in four phases:

1. **Collect** — find all directories containing `.missouri/missouri.yml`
2. **Build nodes** — parse each config, assign state IDs, merge environment
   variables (project env is the base, state env overrides)
3. **Resolve edges** — resolve each transition's relative `target` path to a
   state ID, building the adjacency list
4. **Resolve assertions** — attach assertion commands to their states

Root states (those with no inbound transitions) are the starting points for
path enumeration. Missouri finds all simple paths (no repeated states) from
each root via depth-first search. Each path becomes an independent test
execution.

## How paths run

For each test path, execution proceeds step by step:

1. **Copy to temp** — the source state's files are copied into a fresh temp
   directory. The `.missouri/` config directory is excluded from the copy.
   Directories named `dot-<name>/` inside `.missouri/` are restored as
   `.<name>/` in the temp dir — this is how fixtures carry dotfile state
   like `.git/` or `.clc/` that can't be tracked directly by git.

2. **Run the command** — the transition's shell command executes inside the
   temp directory with a controlled environment (`env_clear` + explicit env
   vars + constructed PATH). The command sees only what was declared.

3. **Diff the result** — the temp directory's contents are recursively
   compared against the target state directory. Both directory trees are
   walked, the `.missouri/` directory is excluded from both sides, and
   project-level ignore patterns (gitignore syntax, from
   `.missouri/ignore`) filter out files that shouldn't be compared. Files
   are compared byte-for-byte. Any difference — extra file, missing file,
   content mismatch — is a failure.

4. **Chain forward** — if the path has more steps, the temp directory (now
   modified by the command) becomes the working directory for the next
   transition. The output of one step is the input of the next.

Paths execute in parallel using rayon. Each path gets its own temp directory,
so there's no shared mutable state between paths.

## Why `env_clear`

Every command execution starts with `env_clear()`. The process inherits
nothing from the host environment — no `HOME`, no `LANG`, no `EDITOR`,
nothing. Environment variables come from three sources only: the project-level
`env` config, the state-level `env` config (which overrides project-level),
and `PATH` (constructed from the project's `bin/` directory and system paths).

A test that passes because `$TERM` happens to be set on the developer's
machine and fails in CI is a coincidence, not a test. `env_clear` forces
every needed variable to be declared explicitly in the test config.

## What gets compared, what doesn't

**Compared by default:**
- Every file and directory in both the actual (temp dir after command) and
  expected (target state directory) trees
- File contents are compared byte-for-byte
- Extra files in actual → failure
- Missing files (in expected but not actual) → failure
- Content mismatches → failure with diff output

**Excluded automatically:**
- The `.missouri/` config directory on both sides
- Files matching patterns in `.missouri/ignore` (gitignore syntax —
  `*.log`, `__pycache__/`, etc.)

**Excluded per-transition:**
- File comparator overrides with `ignore: true` skip specific paths
- Directory comparators (trailing `/`) skip entire subtrees

**Custom comparison:**
- File comparator overrides with a `command` run that command with the
  actual and expected file paths as arguments. Exit 0 means match.
- Env var comparators work the same way for environment variables
- Network comparators handle HTTP request matching for transitions using
  mitmproxy interception

**Environment variables:**
- Compared only when the target state defines `env` or the transition
  defines env comparators. If the target state has no env config, env
  comparison is skipped entirely.

**Stdout and stderr:**
- Compared only when `stdout` or `stderr` fields are set on the transition
  or assertion config. Exact string match.

## Assertions beyond filesystem state

Some properties can't be captured as files on disk. A state's
`.missouri/missouri.yml` can declare `assertions` — shell commands that run
against the state's files and pass or fail based on exit code. Assertions
can also declare expected `stdout` and `stderr` for exact-match comparison,
and `should_fail: true` to assert non-zero exit.

Assertions run *inside* a temp copy of the state, so they can't
accidentally modify the fixture. They're useful for verifying computed
properties: "does `jq` parse this file successfully," "does this command
print the expected output," "does this config file contain the right key."

## How missouri relates to other test approaches

**Unit tests** verify individual functions. Missouri doesn't replace them —
it tests the assembled tool from the outside.

**Integration test harnesses** (writing Rust tests that call your CLI
binary) work, but they push fixture setup into code. The fixture is
whatever the test function creates programmatically. Missouri inverts this:
fixtures are real directories you can inspect, copy, and diff outside of any
test framework.

**Snapshot testing** captures output and compares against stored snapshots.
Missouri is a kind of snapshot testing where the "snapshot" is an entire
directory tree and transitions between snapshots are part of the model.

**Docker-based e2e tests** provide isolation but carry container overhead
and can't easily model state graphs. Missouri's isolation comes from temp
directories and `env_clear`, which is cheaper and doesn't require a container
runtime. For stronger isolation, missouri supports nix shell sandboxes
and microsandbox microVMs.

## Getting started

For setup and first-test walkthrough, see [Getting Started](/missouri/getting-started).
For CLI usage, see [CLI Reference](/missouri/cli-reference).
