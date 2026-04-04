# Architecture

How the crates relate and what depends on what.

## Dependency graph

```
clc ─────┬── tisket ──── mdstore
         ├── missouri ── clc-sdk ── claude-code
         ├── almanac
         ├── belmont
         ├── zettel ──── mdstore
         ├── clc-sdk
         └── claude-code

clc-api ──── tisket ──── mdstore

clc-web      (no workspace deps, talks to clc-api over HTTP)
moose        (no workspace deps)
```

Six crates have no workspace dependencies: almanac, belmont,
claude-code, clc-web, mdstore, moose. These are leaves. Everything
else builds upward from them.

## Layers

### Foundation: mdstore, claude-code

**mdstore** parses and serializes YAML-frontmatter markdown documents.
`Document<T>` is generic over frontmatter type. Tisket and zettel both
store their data as markdown files with typed frontmatter; mdstore is
the shared parser they build on.

**claude-code** defines serde types for Claude Code's NDJSON streaming
protocol. It's a pure data crate with no runtime behavior. clc-sdk
re-exports its types.

### Standalone tools: almanac, belmont, moose

These have no workspace dependencies and no dependents within the
workspace. They solve one problem each and interact with the rest of
the system through the filesystem and CLI.

**almanac** indexes skills. clc calls `almanac` as a library to
build the skill index for prime text injection, but almanac doesn't
know about clc.

**belmont** manages secrets. clc detects `belmont.yml` and injects
status, but belmont doesn't know about clc.

**moose** automates browsers. It's used by agents during testing but
has no compile-time relationship to anything else in the workspace.

### Data tools: tisket, zettel

Both depend on mdstore for their storage layer. Both store data as
markdown files in git.

**tisket** is the issue tracker. clc depends on it for pickup, status
management, and coordinator dispatch. clc-api wraps it in an HTTP
layer.

**zettel** is the knowledge base. clc depends on it for note status
injection into prime text.

### SDK: clc-sdk

Traits that define how tools and workspaces integrate with the
workflow engine.

- `ClcTool` — how a tool reports status and phase-aware directives.
  Implemented by almanac, belmont, tisket, zettel, and missouri
  (via their state structs in clc).
- `Workspace` — how an isolation backend creates and manages
  environments. Implemented by git worktrees and Docker containers.
- `Agent` — how a coding agent process is spawned and configured.
  Currently only Claude Code.

clc-sdk depends on claude-code for protocol types. missouri depends
on clc-sdk so it can implement `ClcTool`.

### Orchestrator: clc

clc is the top of the dependency tree. It depends on every tool
crate and the SDK. It's responsible for:

- Hook system (intercepts every Claude Code event)
- Phase enforcement (evaluates tool calls against workflow permissions)
- Prime text assembly (calls each tool's `prime()` method, composes
  the full orientation text)
- Worktree/workspace management (creates, tracks, cleans up)
- Orchestration (supervisor, coordinators, workers, permissions)
- Configuration (clc.yml parsing, workflow resolution, policy rules)

### Web layer: clc-api, clc-web

**clc-api** is an Axum HTTP server that wraps tisket in REST
endpoints. It depends on tisket only.

**clc-web** is a Leptos CSR frontend that talks to clc-api. It has
no workspace dependencies; it communicates over HTTP.

## Key interfaces

### The ClcTool trait

Every tool that contributes to an agent's context implements
`ClcTool`. The trait has three methods:

- `prime(&self, ctx: &PrimeContext) -> String` — full context
  section for session start. Phase-aware: the output can change
  based on the current workflow phase.
- `status_basic(&self) -> String` — one-liner for periodic
  reinforcement (injected on every UserPromptSubmit).
- `status_full(&self) -> String` — detailed status for diagnostics.

clc assembles prime text by calling `prime()` on each tool's state
struct and concatenating the results. The order is: workflow context,
tisket, missouri, zettel, almanac, belmont.

### The Workspace trait

Abstracts over isolation backends. The key lifecycle:

1. `create()` — provision the environment (create worktree, start
   container)
2. The agent works inside the workspace
3. `cleanup()` — tear down (delete worktree, stop container)

Git worktrees create a branch and a directory. Docker workspaces
build an image, start a container, transfer code via git, and
communicate back through an SSH tunnel and HTTP API.

### Hook event flow

Every agent event flows through `clc hook`:

1. Claude Code fires an event (PreToolUse, PostToolUse,
   UserPromptSubmit, SessionStart, Stop)
2. `clc hook` reads the event from stdin
3. It loads the current git state (branch, is_main, is_worktree)
4. It loads the phase state (if in a worktree with active work)
5. It evaluates the event against the current phase's permissions
6. It returns Allow (with optional injected text), Block (with
   explanation), or Passthrough

SessionStart and UserPromptSubmit always return Allow with injected
context. PreToolUse may Block. PostToolUse may inject nudges. Stop
may Block if the current phase doesn't allow stopping.

## Build

Rust workspace. All crates build together:

```
cargo build --workspace
cargo test --workspace
```

clc is the main binary. tisket, missouri, almanac, belmont, zettel,
and moose are also binaries. clc bundles them as subcommands (`clc
tisket`, `clc missouri`, etc.) but they can be built and run
independently.
