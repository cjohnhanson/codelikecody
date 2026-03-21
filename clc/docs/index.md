<!-- metadata
title: "codelikecody"
description: "Workflow enforcement, issue tracking, and testing for AI coding agents"
type: guide
-->

# codelikecody

AI coding agents are good at writing code. They're less good at writing code *the way you'd want it written* — with tests first, in isolated branches, with clean commit histories and verifiable output. Left to their own devices, agents skip tests, edit trunk directly, and produce work that's hard to review because it happened in one big shot.

codelikecody fixes this by constraining what the agent can do at each step. In `tests-unwritten`, source files are locked — the agent can't edit them until test files exist. Trunk is read-only, so work happens in isolated [git worktrees](https://git-scm.com/docs/git-worktree). And when the agent tries to stop, the hook blocks until the current [phase](/clc/phase-system) reaches a natural pause point. None of this relies on the agent's judgment — it's enforced at the tool-call level by hooking into [Claude Code](https://docs.anthropic.com/en/docs/claude-code)'s event system.

Three tools, each independently useful:

**[clc](/what-is-codelikecody)** enforces workflow. It hooks into Claude Code and intercepts every tool call, gating what's allowed based on the current [phase](/clc/phase-system). Phases progress from `tests-unwritten` through `implementing` to `done`, with file-edit restrictions at each stage. clc also handles [multi-agent orchestration](/clc/orchestration) — dispatching workers to tiskets in parallel.

**[tisket](/tisket/workflow)** tracks issues. Issues are markdown files with YAML frontmatter, stored in `.tisket/` and versioned in git. No server, no database. When an agent picks up a tisket, clc creates an isolated worktree and wires the issue's context into the agent's session — including [scratch notes](/tisket/what-is-tisket), which serve as the agent's persistent working memory across context compaction and session boundaries.

**[missouri](/missouri/getting-started)** runs tests. You put the files you expect to exist in a directory, write a command that should produce them, and missouri checks that the output matches. It handles the scaffolding — temp directories, file comparison, parallel execution — so tests are just directories of expected output. Built for CLI tools and workflow systems where the interesting question is "run this command, check what happened to the files."

## Start here

| If you want to... | Go to |
|---|---|
| Understand the design and philosophy | [What is codelikecody?](/what-is-codelikecody) |
| Set up clc on a project | [Getting Started](/getting-started) |
| Write tests with missouri | [Getting Started with Missouri](/missouri/getting-started) |
| Track issues with tisket | [Tisket Workflow](/tisket/workflow) |
| Look up a specific command | [clc](/clc/cli-reference), [tisket](/tisket/cli-reference), or [missouri](/missouri/cli-reference) CLI reference |
| Understand the phase system | [The Phase System](/clc/phase-system) |
| Run multiple agents in parallel | [Multi-Agent Orchestration](/clc/orchestration) |
| Write complex missouri test suites | [Writing Tests](/missouri/writing-tests) |

## Install

```sh
git clone https://github.com/codelikecody/codelikecody.git
cd codelikecody
cargo build --workspace
export PATH="$PWD/target/debug:$PATH"
```

This builds all three tools. Use whichever ones you need — `tisket` and `missouri` have no dependency on `clc` or each other.

## License

MIT
