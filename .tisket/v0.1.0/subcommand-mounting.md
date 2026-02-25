---
title: "Subcommand mounting"
status: discovery
priority:
assignee:
labels: [architecture]
depends_on: [workspace-restructuring]
created: "2026-02-24T14:52:06Z"
updated: "2026-02-24T14:52:06Z"
---

Mount tisket and missouri CLIs as clc subcommands so `clc tisket <cmd>` and
`clc missouri <cmd>` work identically to the standalone binaries.

## Approach

Each tool exports its clap `Command` or `Cli` struct from lib.rs. The standalone
binary becomes a thin wrapper that calls the same entry point.

clc adds `Tisket` and `Missouri` variants to its own `Command` enum, each
wrapping the tool's exported clap type. Dispatch calls the tool's run function.

## Example

`clc tisket issue list` behaves identically to `tisket issue list`.

## Changes needed

1. tisket: export clap struct and a `run(cli)` function from lib.rs
2. missouri: export clap struct and a `run(cli)` function from lib.rs
3. clc cli.rs: add `Tisket` and `Missouri` subcommand variants
4. clc main.rs: dispatch to tisket/missouri run functions
5. Standalone binaries: refactor to call the shared entry point
