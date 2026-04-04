Do not write or modify LLM prompt content without explicit user approval. Propose drafts eagerly, but do not write to file until approved.

Prompt content in this project includes:
- AGENTS.md / CLAUDE.md (agent instructions loaded at session start)
- Prime text (imperative directives injected by clc hooks)
- Hook context injection (SessionStart, reinforcement, Stop messages)
- Documentation that agents consume (docs bundled in the binary)

## CRITICAL: Test failures and compiler warnings

**Test failures and compiler warnings are NEVER acceptable. There are
no exceptions to this rule.**

- Never dismiss a failing test as "pre-existing" or "unrelated to this change."
- Never dismiss a compiler warning as "from another branch" or "not in scope."
- Never say "that's not ours to fix." If it's in the build output, it's yours to fix.
- Never proceed with other work while tests are failing or warnings exist.
- `cargo build --workspace` must produce zero warnings.
- `cargo test --workspace` failures must be investigated and fixed or captured as a tisket.

Every failure. Every warning. Every time. Work is not complete until
the build is clean and tests pass. If something broke from a merge,
fix it — the merge made it yours.

## Deploying code changes

Three separate binaries must stay in sync. Each is updated differently,
and using a stale one causes silent failures (wrong behavior, missing
CLI flags, old Docker images).

### 1. Workspace binary (cargo build)

`cargo build --workspace` builds to `./target/debug/`. Use this for
local testing. This is the fastest path — seconds, not minutes.

### 2. System binary (nix)

`clc`/`tisket`/`missouri` on PATH come from nix. Updated by:

    cd ~/Projects/co.d
    git add -A && git commit -m "update"
    nix flake update codelikecody
    hms

Takes ~2 minutes. Required when: `clc up` is run from the terminal
(the supervisor uses the system binary), or any manual CLI usage.

### 3. Docker image (depot)

Worker and coordinator containers run binaries baked into the Docker
image. Updated by:

    depot build --platform linux/arm64 -t clc-worker:latest --load -f docker/worker/Dockerfile .

Takes ~10 minutes. The build context MUST be the repo root (`.`), not
`docker/worker/`. The `-f` flag points to the Dockerfile. Required
when: any code change affects behavior inside containers (CLI flags,
API endpoints, phase logic, guard rules).

### The full sequence after landing changes

After merging to main, if the change affects runtime behavior:

1. `git push` — remote has the code
2. `cargo build --workspace` — local binary for testing
3. `depot build ...` — Docker image for containers
4. `cd ~/Projects/co.d && nix flake update codelikecody && hms` — system binary
5. Only then: `clc up`

Steps 3 and 4 can run in parallel. Skipping any step means that
binary is stale. The most common failure: Docker image built from
commit N, system binary from commit N+1 (or vice versa). The
coordinator inside Docker rejects flags the supervisor sends.

## Skills

Two skill directories serve different audiences:

- **`skills/`** — Skills for agents *using* this project's tools (missouri,
  tisket, clc). Product documentation for consumers. These ship with the
  project and teach agents how to use what's built here.

- **`.agents/skills/`** — Skills for agents *developing* this project.
  Internal conventions, development practices, repo-specific patterns.
  Symlinked to `.claude/skills/` for Claude Code integration.

### Continuous improvement

When a skill causes friction — wrong instructions, missed cases, the user
corrects something a skill recommended — update the skill immediately.
Skills rot. Commands get renamed, patterns evolve, tools change defaults.
A skill that was correct last month may not be correct now. When touching
a skill for any reason, verify its claims still hold. If the fix is
non-trivial, create a tisket to track it.

## Documentation completeness

When adding or changing user-facing features (new commands, config fields,
assertion types, API changes), the following must be updated before the
work is considered done:

- **Bundled docs** (`missouri/docs/`, `tisket/docs/`, etc.) — CLI reference,
  guides, and conceptual docs shipped with the binary
- **Product skills** (`skills/`) — skills that teach agents how to use the tools
- **Development skills** (`.agents/skills/`) — if the change affects how
  agents develop this project

Documentation is not a follow-up. It ships with the code.

## Binary vs project configuration

Code compiled into clc/tisket/missouri/etc binaries must be
project-agnostic. Do not put project-specific configuration into source
code. Things unique to a particular project's setup belong in that
project's configuration files:

- **Docker image** — toolchains, dependencies, system packages
- **clc.yaml** — topology, coordinators, workflows, selectors
- **Environment configuration** — PATH, env vars, shell setup
