<!-- metadata
title: "codelikecody"
description: "Workflow enforcement, issue tracking, and testing for AI coding agents"
type: guide
-->

# codelikecody

Codelikecody is three tools for managing coding agent workflows. They work together or independently.

## The tools

### clc — workflow enforcement

clc hooks into Claude Code and enforces a phase-gated TDD cycle: write tests, see them fail, implement, get green. Agents work in isolated git worktrees, and the hook system blocks actions that violate the current phase. It also handles worker orchestration — dispatching, monitoring, and landing multiple agent processes.

[Getting started](getting-started.md) -- [CLI reference](clc/cli-reference.md)

### tisket — plaintext issue tracking

Issues as markdown files with YAML frontmatter, stored in `.tisket/` and versioned in git. No database, no server. Git-aware divergence detection, full-text search, scratch notes for agent working memory. Works fine without clc — it's a standalone issue tracker.

[CLI reference](tisket/cli-reference.md)

### missouri — filesystem state graph testing

Define expected filesystem states as directories, connect them with shell commands as transitions, and missouri walks every path through the graph, running commands in sandboxed temp directories and diffing the results. Built for testing CLI tools and workflow systems. No dependency on clc or tisket.

[Getting started](missouri/getting-started.md) -- [CLI reference](missouri/cli-reference.md)

## If you want to...

- **...understand the philosophy** — [What is codelikecody?](what-is-codelikecody.md)
- **...set up clc on a project** — [Getting started](getting-started.md)
- **...use missouri for testing** — [Getting started with missouri](missouri/getting-started.md)
- **...track issues with tisket** — [tisket CLI reference](tisket/cli-reference.md)

## Install

Build from source (requires Rust stable toolchain):

```sh
git clone https://github.com/codelikecody/codelikecody.git
cd codelikecody
cargo install --path clc
cargo install --path tisket
cargo install --path missouri
```

Install only what you need. `tisket` and `missouri` have no dependency on `clc` or each other.

## License

MIT
