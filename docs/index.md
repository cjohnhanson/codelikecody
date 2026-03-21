<!-- metadata
title: "codelikecody"
description: "Workflow enforcement, issue tracking, and testing for AI coding agents"
type: guide
-->

# codelikecody

AI coding agents are good at writing code. They're less good at writing code *the way you'd want it written* — with tests first, in isolated branches, with clean commit histories and verifiable output. Left to their own devices, agents skip tests, edit trunk directly, and produce work that's hard to review because it happened in one big shot.

codelikecody is a set of tools that mechanically enforce the workflow you'd use if you were pair-programming with an infinitely patient but occasionally reckless junior developer. The agent can't skip tests because the hook system won't let it edit source files until tests exist. It can't touch trunk because trunk is read-only. It can't stop working because the stop hook blocks until the phase reaches a natural pause point.

Three tools, each independently useful:

**[clc](what-is-codelikecody.md)** enforces workflow. It hooks into Claude Code and intercepts every tool call. In `tests-unwritten`, the agent can't edit source files. In `implementing`, everything's unlocked. In `green`, source files lock again. The agent doesn't choose to follow TDD — the hook system makes it the only option.

**[tisket](tisket/workflow.md)** tracks issues. Issues are markdown files with YAML frontmatter, stored in `.tisket/` and versioned in git. No server, no database. When an agent picks up a tisket, clc creates an isolated worktree and wires the issue's context — title, body, scratch notes — into the agent's session. Scratch notes are the agent's working memory across context boundaries.

**[missouri](missouri/getting-started.md)** runs tests. You put the files you expect to exist in a directory, write a command that should produce them, and missouri checks that the output matches. It handles the scaffolding — temp directories, file comparison, parallel execution — so tests are just directories of expected output. Built for CLI tools and workflow systems where the interesting question is "run this command, check what happened to the files."

## Start here

| If you want to... | Go to |
|---|---|
| Understand the design and philosophy | [What is codelikecody?](what-is-codelikecody.md) |
| Set up clc on a project | [Getting Started](getting-started.md) |
| Write tests with missouri | [Getting Started with Missouri](missouri/getting-started.md) |
| Track issues with tisket | [Tisket Workflow](tisket/workflow.md) |
| Look up a specific command | [clc](clc/cli-reference.md), [tisket](tisket/cli-reference.md), or [missouri](missouri/cli-reference.md) CLI reference |
| Understand the phase system | [The Phase System](clc/phase-system.md) |
| Run multiple agents in parallel | [Multi-Agent Orchestration](clc/orchestration.md) |
| Write complex missouri test suites | [Writing Tests](missouri/writing-tests.md) |

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
