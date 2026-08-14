# Development

This document describes how to work on this codebase.

## Prerequisites

- Rust (stable)
- [nix](https://nixos.org/) — optional. Use it to run tests in a
  sandbox.

## Build

```
cargo build --workspace
cargo test --workspace
```

Both commands must produce zero warnings and zero failures. There are
no exceptions. Read [CLAUDE.md](CLAUDE.md) for the full policy.

## Project structure

```
clc/                 # workflow engine binary
clc-sdk/             # SDK traits (Workspace, ClcTool, Agent)
clc-api/             # HTTP API for tisket
clc-web/             # Leptos frontend
tisket/              # issue tracker binary
missouri/            # test framework binary
almanac/             # skill aggregator binary
belmont/             # secret manager binary
zettel/              # knowledge base binary
mdstore/             # markdown document store library
claude-code/         # Claude Code protocol types library
skills/              # product skills (ship with the binary)
.agents/skills/      # development skills (for working on this repo)
```

Read [ARCHITECTURE.md](ARCHITECTURE.md) to see how the crates depend on
each other.

## Testing with missouri

Most crates have a missouri test suite under `tests/missouri/`. These
are state graph tests, not Rust unit tests. Run them with these
commands:

```
missouri run                  # run all paths
missouri run -v               # verbose output
missouri list --show paths    # see what paths exist
```

missouri tests live next to the Rust tests. `cargo test --workspace`
runs both. Test modules connect the missouri tests to the Rust test
harness.

## The workflow (when using clc)

Follow these steps when clc manages the work:

1. `tisket issue list` — find or create the work.
2. `clc pickup <id>` — create a workspace and set the first phase.
3. Write the tests, write the code, and make the tests pass.
4. `clc done` — close the tisket.
5. `clc merge <id>` — merge to trunk.

The phase system enforces this workflow. Read `clc docs phase-system`
for the details.

The same conventions apply when you work without clc, for example on
clc itself with `CLC_GUARD_OFF=1`. Write the tests first. Keep the
build clean. Allow no warnings.

## Documentation

There are three kinds of documentation. Update all three when you
change user-facing behavior:

1. **Bundled docs** (`<crate>/docs/`) — these ship with the binary.
   Read them with `<tool> docs [topic]`. They use the Diataxis types:
   tutorial, how-to guide, reference, and explanation.

2. **Product skills** (`skills/`) — these teach agents how to use the
   tools. almanac compiles them into the binary.

3. **Development skills** (`.agents/skills/`) — these teach agents how
   to develop this project. They are symlinked to `.claude/skills/`.

## Skills

A skill is a directory with a SKILL.md file. There are two skill
directories:

- `skills/` — for users of the tools. This is product documentation.
- `.agents/skills/` — for developers of the tools. This holds the
  internal conventions.

Update a skill when it is wrong. Create a skill when one is missing.
Skills go out of date faster than code does.

## Stale binaries

The system binary on PATH can be out of date after you land changes.
Run `cargo build --workspace` before you test by hand, and before you
start a coordinator or a worker. `clc dispatch` uses the binaries that
are on PATH.
