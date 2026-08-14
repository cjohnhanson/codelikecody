# Architecture

This document describes how the crates relate and what depends on what.

> **Status (2026-08):** clc is mothballed. This repo no longer runs
> under clc hooks. [gaff](https://github.com/cjohnhanson/gaff) now
> supplies session context and reminders. The dependency graph below
> predates the repo split. almanac, belmont, tisket, zettel, and
> mdstore now live in their own repos, and this repo consumes them as
> git dependencies. The graph remains here to describe the code that
> this repo still holds: clc, clc-sdk, clc-api, clc-web, claude-code,
> and missouri.

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
```

Five crates have no workspace dependencies: almanac, belmont,
claude-code, clc-web, and mdstore. These crates are the leaves of the
graph. Every other crate builds on them.

## Layers

### Foundation: mdstore, claude-code

**mdstore** parses and serializes markdown documents with YAML
frontmatter. `Document<T>` is generic over the frontmatter type. tisket
and zettel both store their data as markdown files with typed
frontmatter. mdstore is the shared parser under both tools.

**claude-code** defines serde types for Claude Code's NDJSON streaming
protocol. It holds data types only and has no runtime behavior. clc-sdk
re-exports its types.

### Standalone tools: almanac, belmont

These crates have no workspace dependencies, and no workspace crate
depends on them. Each one solves a single problem. Each one talks to the
rest of the system through the filesystem and the CLI.

**almanac** indexes skills. clc calls almanac as a library to build the
skill index for prime text injection. almanac knows nothing about clc.

**belmont** manages secrets. clc detects `belmont.yml` and injects the
belmont status. belmont knows nothing about clc.

### Data tools: tisket, zettel

Both crates depend on mdstore for storage. Both store data as markdown
files in git.

**tisket** is the issue tracker. clc uses it for pickup, status
management, and coordinator dispatch. clc-api wraps it in an HTTP layer.

**zettel** is the knowledge base. clc uses it to inject note status into
prime text.

### SDK: clc-sdk

These traits define how tools and workspaces connect to the workflow
engine.

- `ClcTool` — reports status and phase-aware directives. almanac,
  belmont, tisket, zettel, and missouri implement it through their
  state structs in clc.
- `Workspace` — creates and manages an isolation environment. Git
  worktrees and Docker containers implement it.
- `Agent` — spawns and configures a coding agent process. Only Claude
  Code implements it today.

clc-sdk depends on claude-code for the protocol types. missouri depends
on clc-sdk so that it can implement `ClcTool`.

### Orchestrator: clc

clc sits at the top of the dependency tree. It depends on every tool
crate and on the SDK. clc handles:

- The hook system. It intercepts every Claude Code event.
- Phase enforcement. It checks each tool call against the workflow
  permissions.
- Prime text assembly. It calls each tool's `prime()` method and builds
  the full orientation text.
- Worktree and workspace management. It creates, tracks, and removes
  them.
- Orchestration of supervisors, coordinators, workers, and permissions.
- Configuration. It parses clc.yml, resolves workflows, and applies
  policy rules.

### Web layer: clc-api, clc-web

**clc-api** is an Axum HTTP server. It wraps tisket in REST endpoints.
It depends on tisket only.

**clc-web** is a Leptos client-side frontend. It talks to clc-api over
HTTP and has no workspace dependencies.

## Key interfaces

### The ClcTool trait

Every tool that adds to an agent's context implements `ClcTool`. The
trait has three methods:

- `prime(&self, ctx: &PrimeContext) -> String` — returns the full
  context section for session start. The output changes with the
  current workflow phase.
- `status_basic(&self) -> String` — returns one line for periodic
  reinforcement. clc injects it on every UserPromptSubmit.
- `status_full(&self) -> String` — returns detailed status for
  diagnostics.

clc builds the prime text in one pass. It calls `prime()` on each tool's
state struct and joins the results. The order is workflow context,
tisket, missouri, zettel, almanac, belmont.

### The Workspace trait

This trait covers the isolation backends. The lifecycle has three
steps:

1. `create()` — prepare the environment. Create the worktree or start
   the container.
2. The agent works inside the workspace.
3. `cleanup()` — remove the environment. Delete the worktree or stop
   the container.

A git worktree backend creates a branch and a directory. A Docker
backend builds an image, starts a container, and transfers the code
through git. It then talks back through an SSH tunnel and an HTTP API.

### Hook event flow

Every agent event passes through `clc hook`:

1. Claude Code fires an event: PreToolUse, PostToolUse,
   UserPromptSubmit, SessionStart, or Stop.
2. `clc hook` reads the event from stdin.
3. It loads the current git state: branch, is_main, and is_worktree.
4. It loads the phase state if the session runs in a worktree with
   active work.
5. It checks the event against the permissions of the current phase.
6. It returns Allow, Block, or Passthrough. Allow can carry injected
   text. Block carries an explanation.

SessionStart and UserPromptSubmit always return Allow with injected
context. PreToolUse can return Block. PostToolUse can inject a
reminder. Stop returns Block when the current phase forbids stopping.

## Build

This repo is a Rust workspace. All crates build together:

```
cargo build --workspace
cargo test --workspace
```

clc is the main binary. tisket, missouri, almanac, belmont, and zettel
are binaries too. clc bundles them as subcommands, such as `clc tisket`
and `clc missouri`. You can also build and run each one on its own.
