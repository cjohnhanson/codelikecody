<!-- metadata
title: "What is codelikecody?"
description: "Philosophy, tools, and how they fit together"
type: explanation
-->

# What is codelikecody?

Codelikecody is a workflow engine for coding agents. It manages the lifecycle of agent work — from issue tracking through test-driven development to completion — using hook-based enforcement and git worktree isolation.

The project is three tools: **clc**, **tisket**, and **missouri**. They're designed to work together, but each one stands on its own.

## The tools

### clc — workflow enforcement

clc is the orchestrator. It runs as a hook system inside Claude Code, intercepting every event in the agent's session: tool calls, prompt submissions, stop attempts. Based on the current git branch and workflow phase, it decides what's allowed and what's blocked. See the [clc CLI reference](clc/cli-reference.md) for the full command surface.

The core mechanism is a phase system. Work progresses through an ordered sequence:

`tests-unwritten` → `tests-written` → `red` → `implementing` → `green` → `review-requested` → `reviewed` → `done`

Phases aren't advisory. In `tests-unwritten`, the hook will reject edits to source files. The agent can only write tests. Once tests exist and fail (`red`), the phase advances to `implementing` and source edits are unlocked. The agent can't stop the session until it reaches `review-requested` or later (stop is blocked at `green` — the agent has to advance past it).

This is TDD enforced mechanically, not by asking nicely.

clc also manages worktree isolation. All implementation happens in git worktrees, never on trunk. Trunk is read-only — the hooks block file writes and restrict Bash to a conservative allowlist. An agent that wants to change code has to pick up a tisket first, which creates a worktree and sets the initial phase.

### tisket — plaintext issue tracking

Tisket is an issue tracker where issues are markdown files with YAML frontmatter, stored in `.tisket/` directories within the repository. No database, no server, no API. Issues are just files, versioned in git alongside the code they describe.

Issues follow a status lifecycle: `discovery` → `todo` → `in_progress` → `done`. New issues start in `discovery` — an idea captured, not yet scoped. Promotion to `todo` is a deliberate decision that the issue has enough detail for an agent (or a person) to pick it up and work it without guessing.

Tisket provides git-aware divergence detection (has the issue changed since this branch diverged?) and full-text search across titles, bodies, and scratch notes. The scratch notes section on each issue serves as the agent's working memory — the only persistent state that survives context compaction and session boundaries. See the [tisket CLI reference](tisket/cli-reference.md) for the full command and schema details.

### missouri — filesystem state graph testing

Missouri is an end-to-end test framework built around a specific model: directed graphs of filesystem states.

Each state is a directory. The directory contains the files that should exist in that state, plus a `.missouri/missouri.yml` config that defines transitions (shell commands that move to other states) and assertions (shell expressions that must hold). Missouri walks every path through the graph, executing transitions in sandboxed temp directories and comparing the resulting filesystem against the expected state.

This model is a natural fit for testing CLI tools and workflow systems, where the interesting behavior is "run this command, check that these files changed in these ways." Missouri handles the scaffolding — temp dirs, file comparison, path enumeration, parallel execution — so tests are just directories of expected output. The [missouri getting started tutorial](missouri/getting-started.md) walks through building a test suite from scratch, and the [CLI reference](missouri/cli-reference.md) covers the full config schema.

## How they fit together

The three tools form a loop:

1. **Tisket** holds the work to be done. An issue describes what needs building, what done looks like, and (once work starts) scratch notes tracking progress.

2. **clc** picks up a tisket, creates an isolated worktree, and sets the agent's phase to `tests-unwritten`. From here, the phase system enforces the TDD cycle: write tests, watch them fail, implement, get green.

3. **Missouri** is where the tests live. clc's test infrastructure for its own workflow commands is built on missouri — states represent project configurations (a repo with a tisket, a repo in the `implementing` phase, etc.) and transitions are the CLI commands being tested.

4. When tests pass and the phase reaches `green`, the agent finalizes with `clc done`, which handles the bookkeeping of marking the tisket complete and cleaning up.

clc detects the presence of both tisket and missouri in the working directory and injects their status into the agent's context at session start and on every prompt. The agent always knows which tisket it's working on, what phase it's in, and whether missouri tests exist.

## What it looks like in practice

The short version: trunk is read-only. An agent picks up a tisket, which creates an isolated worktree and locks the phase to test-writing. Tests first, then implementation, then finalization. Every tool call passes through clc's hooks — the agent doesn't need to remember the rules.

The [getting started tutorial](getting-started.md) walks through the full cycle end-to-end.

