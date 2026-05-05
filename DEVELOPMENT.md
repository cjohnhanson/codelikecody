# Development

How to work on this codebase.

## Prerequisites

- Rust (stable)
- [nix](https://nixos.org/) (optional, for sandboxed test execution)

## Build

```
cargo build --workspace
cargo test --workspace
```

Both must produce zero warnings and zero failures. No exceptions.
See [CLAUDE.md](CLAUDE.md) for the full policy.

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

See [ARCHITECTURE.md](ARCHITECTURE.md) for how crates depend on each
other.

## Testing with missouri

Most crates have missouri test suites under `tests/missouri/`. These
are state graph tests, not Rust unit tests. Run them with:

```
missouri run                  # run all paths
missouri run -v               # verbose output
missouri list --show paths    # see what paths exist
```

Missouri tests live alongside Rust tests. Both run via `cargo test
--workspace` (missouri tests are wired into the Rust test harness via
test modules).

## The workflow (when using clc)

If you're using clc to manage work:

1. `tisket issue list` — find or create work
2. `clc pickup <id>` — creates a workspace, sets initial phase
3. Write tests, implement, get green
4. `clc done` — close the tisket
5. `clc merge <id>` — merge to trunk

The phase system enforces the workflow. See `clc docs phase-system`
for details.

If you're working without clc (e.g., on clc itself with
`CLC_GUARD_OFF=1`), the conventions still apply: tests first,
clean build, no warnings.

## Documentation

Three kinds of documentation, all required when changing user-facing
behavior:

1. **Bundled docs** (`<crate>/docs/`) — ship with the binary, accessed
   via `<tool> docs [topic]`. Diataxis types: tutorials, how-to
   guides, reference, explanation.

2. **Product skills** (`skills/`) — teach agents how to use the tools.
   These compile into the binary via almanac.

3. **Development skills** (`.agents/skills/`) — teach agents how to
   develop this project. Symlinked to `.claude/skills/`.

## Skills

Skills are directories with a SKILL.md file. Two directories:

- `skills/` — for users of the tools (product documentation)
- `.agents/skills/` — for developers of the tools (internal conventions)

When a skill is wrong, update it. When a skill is missing, create it.
Skills rot faster than code.

## Stale binaries

After landing changes, the system binary on PATH may be stale. Always
`cargo build --workspace` before manual testing or launching
coordinators/workers. `clc dispatch` uses whatever binaries are on
PATH.
