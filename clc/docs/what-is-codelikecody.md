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

clc is the orchestrator. It runs as a hook system inside Claude Code, intercepting every event in the agent's session: tool calls, prompt submissions, stop attempts. Based on the current git branch and workflow phase, it decides what's allowed and what's blocked. See the [clc CLI reference](/clc/cli-reference) for the full command surface.

The core mechanism is a phase system. Work progresses through an ordered sequence:

`tests-unwritten` → `tests-written` → `red` → `implementing` → `green` → `review-requested` → `in-review` → `reviewed` → `done`

Phases aren't advisory. In `tests-unwritten`, the hook will reject edits to source files — only files under `tests/missouri/` are allowed. Once tests exist and fail (`red`), the phase advances to `implementing` and all edits are unlocked. Source edits lock again after `green`: the `review-requested` and `reviewed` phases restrict edits back to test paths, while `in-review` reopens everything (like `implementing`) so review feedback can be addressed.

The stop hook follows a different logic. An agent can stop at `review-requested`, `reviewed`, or `done` — these are natural pause points. Every other phase blocks the stop event. The agent can't bail at `green`; it has to advance to `review-requested` first. And `in-review` blocks stop too — once review changes are underway, they have to be finished.

This is TDD enforced mechanically, not by asking nicely.

clc also manages worktree isolation. All implementation happens in git worktrees, never on trunk. Trunk is read-only — the hooks block Edit, Write, and NotebookEdit entirely, and restrict Bash to a conservative allowlist (git, cargo, clc, missouri, tisket queries, and a handful of read-only utilities like ls, cat, find). An agent that wants to change code has to pick up a tisket first, which creates a worktree and sets the initial phase.

### tisket — plaintext issue tracking

Tisket is an issue tracker where issues are markdown files with YAML frontmatter, stored in `.tisket/` directories within the repository. No database, no server, no API. Issues are just files, versioned in git alongside the code they describe.

Issues follow a status lifecycle: `discovery` → `todo` → `in_progress` → `done`. New issues start in `discovery` — an idea captured, not yet scoped. Promotion to `todo` is a deliberate decision that the issue has enough detail for an agent (or a person) to pick it up and work it without guessing.

Tisket provides git-aware divergence detection (has the issue changed since this branch diverged?) and full-text search across titles, bodies, and scratch notes. The scratch notes section on each issue serves as the agent's working memory — the only persistent state that survives context compaction and session boundaries. See the [tisket CLI reference](/tisket/cli-reference) for the full command and schema details.

### missouri — filesystem state graph testing

Missouri is an end-to-end test framework built around a specific model: directed graphs of filesystem states.

Each state is a directory. The directory contains the files that should exist in that state, plus a `.missouri/missouri.yml` config that defines transitions (shell commands that move to other states) and assertions (shell expressions that must hold). Missouri walks every path through the graph, executing transitions in sandboxed temp directories and comparing the resulting filesystem against the expected state.

This model is a natural fit for testing CLI tools and workflow systems, where the interesting behavior is "run this command, check that these files changed in these ways." Missouri handles the scaffolding — temp dirs, file comparison, path enumeration, parallel execution — so tests are just directories of expected output. The [missouri getting started tutorial](/missouri/getting-started) walks through building a test suite from scratch, and the [CLI reference](/missouri/cli-reference) covers the full config schema.

## How they fit together

The three tools form a loop:

1. **Tisket** holds the work to be done. An issue describes what needs building, what done looks like, and (once work starts) scratch notes tracking progress.

2. **clc** picks up a tisket, creates an isolated worktree, and sets the agent's phase to `tests-unwritten`. From here, the phase system enforces the TDD cycle: write tests, watch them fail, implement, get green.

3. **Missouri** is where the tests live. clc's test infrastructure for its own workflow commands is built on missouri — states represent project configurations (a repo with a tisket, a repo in the `implementing` phase, etc.) and transitions are the CLI commands being tested.

4. When tests pass and the phase reaches `green`, the agent advances through `review-requested`. The `clc done` command handles the bookkeeping of marking the tisket complete and cleaning up.

clc detects the presence of both tisket and missouri in the working directory and injects their status into the agent's context at session start and on every prompt. The agent always knows which tisket it's working on, what phase it's in, and whether missouri tests exist.

## What it looks like in practice

When an agent session starts, clc fires a `SessionStart` hook and injects a block of prime text. This text isn't a suggestion — it's the agent's operating context. It includes the current branch name, whether the branch is trunk or a worktree, the current phase (if any), the full workflow loop description, TDD mandates, commit discipline rules, and instructions for capturing discovered work as new tiskets. If a tisket matches the current branch, its title, body, and scratch notes are included. If missouri tests exist, their state is injected too.

On trunk, the prime text explains that the branch is read-only and tells the agent how to begin work. The agent can read code, run tests, query tiskets, and use clc commands — but any attempt to edit a file or run an unapproved Bash command gets blocked with an explanation.

Once the agent picks up a tisket, it lands in a worktree with phase `tests-unwritten`. From here, every tool call passes through the guard. Edit a source file? Blocked — write tests first. Try to stop? Blocked — work isn't done. The phase system releases constraints as work progresses: `implementing` unlocks all edits, `green` re-locks source files, `review-requested` is the first point where the agent can stop.

Agents lose the thread. After a long sequence of tool calls, the original task, the current phase, and the test status can all drift out of the agent's effective context. clc compensates by re-injecting orientation context on every prompt submission and nudging the agent to run tests after every file edit during implementation. The agent never has to remember where it is — the system tells it, repeatedly.

The [getting started tutorial](/getting-started) walks through the full cycle end-to-end.

## Multi-agent orchestration

clc supports a coordinator-worker model for running multiple agents in parallel.

The coordinator is an agent that runs on trunk. It doesn't write code — it manages workers that do. A coordinator is launched with `clc coordinate`, which scans for pickable tiskets (status `todo`, all dependencies resolved), builds a prompt listing them, and spawns a coordinator agent process. The coordinator gets a system prompt explaining its role: dispatch workers, monitor their progress, land completed work, dispatch more.

Workers are individual agent sessions, each in its own worktree. The coordinator dispatches them with `clc dispatch`, which runs the same pickup-and-worktree-creation flow as interactive use, then spawns a detached Claude process. When a worker reaches `done`, the coordinator lands it — merging the branch back to trunk — and dispatches the next available tisket. The loop is autonomous.

See the [orchestration guide](/clc/orchestration) for the full workflow: dispatching, monitoring, permissions, and landing.

Workers have limited permissions. When a worker needs a tool that isn't pre-approved, it files a request with `clc permissions request` and stops. The coordinator can grant the request directly or escalate it to the user. Permission policy is configurable: patterns can be auto-granted, always-escalated, or left to the coordinator's judgment. The policy is passed into the coordinator's system prompt so it knows the rules without consulting external state.

Tisket filtering controls what the coordinator sees. The coordinator can be scoped to a specific project, label, dependency chain, or individual tisket. An `--exclude-label` flag allows marking tiskets that shouldn't be auto-dispatched. These filters are evaluated at launch time to build the pickable list.

## What it's not

clc is not a CI/CD system. It doesn't build artifacts, manage deployments, or interact with external services. The hook system runs inside Claude Code sessions — no daemon, no server.

clc does not manage the agent's model, context window, or token budget. It injects text and blocks tool calls — it doesn't control what the agent thinks or how much it costs.

[Missouri](/missouri/what-is-missouri) is the only test framework clc has built-in awareness of. The [phase system](/clc/phase-system) doesn't know about cargo test, pytest, or any other test runner. An agent can run any test command, but the hooks don't parse test output or auto-advance phases based on results. Phase transitions are explicit commands.
