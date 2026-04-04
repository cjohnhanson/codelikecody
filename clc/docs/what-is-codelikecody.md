<!-- metadata
title: "What is codelikecody?"
description: "Philosophy, tools, and how they fit together"
type: explanation
-->

# What is codelikecody?

Codelikecody is a workflow engine for coding agents. It manages the
lifecycle of agent work — from issue tracking through test-driven
development to completion — using hook-based enforcement and workspace
isolation.

The project is seven tools and five supporting libraries. The tools are
designed to work together, but each stands on its own.

## The tools

### clc — workflow enforcement

clc is the orchestrator. It runs as a hook system inside Claude Code,
intercepting every event in the agent's session: tool calls, prompt
submissions, stop attempts. Based on the current git branch and workflow
phase, it decides what's allowed and what's blocked. See the [CLI
reference](/clc/cli-reference) for the full command surface.

The core mechanism is a configurable phase system. Workflows define a
directed graph of phases, each with permissions (what files can be
edited), instructions (injected into agent context), and stop gates
(whether the agent can exit). The default workflow is a TDD sequence:

`tests-unwritten` → `tests-written` → `red` → `implementing` → `green`
→ `review-requested` → `in-review` → `reviewed` → `done`

Custom workflows can define any phase graph. Policy rules in `clc.yml`
select workflows based on issue labels or project, so different kinds
of work can follow different processes.

Phases aren't advisory. A guard intercepts every tool call and evaluates
it against the current phase's permissions. If the permission set says
only test files can be edited, the guard rejects edits to source files
before they reach the filesystem.

clc also manages workspace isolation. Workspace is a trait: git worktrees
and Docker containers are the current backends. All implementation
happens in workspaces, never on trunk. Trunk is read-only — the hooks
block Edit, Write, and NotebookEdit entirely, and restrict Bash to a
conservative allowlist.

### tisket — plaintext issue tracking

Tisket is an issue tracker where issues are markdown files with YAML
frontmatter, stored in `.tisket/` directories within the repository. No
database, no server, no API. Issues are just files, versioned in git
alongside the code they describe.

Issues follow a status lifecycle: `discovery` → `todo` → `in_progress`
→ `done`. New issues start in `discovery` — an idea captured, not yet
scoped. Promotion to `todo` is a deliberate decision that the issue has
enough detail for an agent (or a person) to pick it up and work.

Tisket provides git-aware divergence detection (has the issue changed
since this branch diverged?) and full-text search across titles, bodies,
and scratch notes. The scratch notes section on each issue serves as the
agent's working memory — persistent state that survives context
compaction and session boundaries. See the [tisket CLI
reference](/tisket/cli-reference) for the full command and schema details.

### missouri — filesystem state graph testing

Missouri is an end-to-end test framework built around directed graphs
of filesystem states.

Each state is a directory. The directory contains the files that should
exist in that state, plus a `.missouri/missouri.yml` config that defines
transitions (shell commands that move to other states) and assertions
(shell expressions that must hold). Missouri walks every path through
the graph, executing transitions in sandboxed temp directories and
comparing the resulting filesystem against the expected state.

This model fits CLI tools and workflow systems, where the interesting
behavior is "run this command, check that these files changed in these
ways." The [getting started tutorial](/missouri/getting-started) walks
through building a test suite from scratch, and the [CLI
reference](/missouri/cli-reference) covers the full config schema.

### almanac — skill aggregation

Almanac indexes agent skills from local directories and built-in sources.
A skill is a directory with a SKILL.md file: YAML frontmatter declares
name and description, the body holds instructions. Agents call `almanac
show <name>` to load the full skill content when needed. See the
[CLI reference](/almanac/cli-reference).

### belmont — secret management

Belmont resolves secrets from pluggable backends (OS keychain,
environment variables) and injects them into commands at runtime. It
scrubs secret values from command output so agents can use credentials
without seeing them. The threat model is narrow: prevent common
exfiltration patterns, not defend against adversarial extraction.

### zettel — knowledge base

A zettelkasten for repos. Atomic markdown notes in `.zettel/`, with
tags, links, and status. Agents create draft notes during research;
humans review and promote them. See the [CLI
reference](/zettel/cli-reference).

### moose — browser automation

Browser automation for AI agents. Forked from Vercel Labs'
agent-browser. Connects to Chrome via CDP, with support for Lightpanda
as an alternative engine and WebDriver/Appium for native mobile testing.

## How they fit together

The tools form a loop:

1. **Tisket** holds the work to be done. An issue describes what needs
   building and what done looks like.

2. **clc** picks up a tisket, creates an isolated workspace, and sets
   the initial phase. The phase system enforces the configured workflow:
   in the default TDD workflow, tests come first, then implementation.

3. **Missouri** is where the tests live. States represent project
   configurations, transitions are CLI commands being tested.

4. When the workflow reaches its terminal phase, `clc done` handles
   the bookkeeping: marking the tisket complete and cleaning up the
   workspace.

**Almanac** supplies skills — procedural knowledge agents load on
demand. **Belmont** provides credentials. **Zettel** captures knowledge
discovered during work. **Moose** handles browser testing.

clc detects the presence of each tool in the working directory and
injects their status into the agent's context at session start and on
every prompt. The agent always knows which tisket it's working on, what
phase it's in, what skills are available, and whether tests exist.

## What it looks like in practice

When an agent session starts, clc fires a `SessionStart` hook and
injects prime text. This text includes the current branch name, whether
it's trunk or a workspace, the current phase (if any), the full workflow
description, and instructions. If a tisket matches the current branch,
its title, body, and scratch notes are included. If missouri tests
exist, their state is injected. If almanac skills are available, the
full index is included.

On trunk, the prime text explains that the branch is read-only and tells
the agent how to begin work. The agent can read code, run tests, query
tiskets, and use clc commands — but any attempt to edit a file or run an
unapproved Bash command gets blocked with an explanation.

Once the agent picks up a tisket, it lands in a workspace with the
workflow's initial phase. From here, every tool call passes through the
guard. The phase system releases constraints as work progresses according
to the configured workflow.

Agents lose the thread. After a long sequence of tool calls, the
original task, the current phase, and the test status can all drift out
of effective context. clc compensates by re-injecting orientation on
every prompt submission and nudging the agent after file edits. The
agent never has to remember where it is — the system tells it,
repeatedly.

The [getting started tutorial](/clc/getting-started) walks through the
full cycle end-to-end.

## Multi-agent orchestration

clc supports a supervisor/coordinator/worker hierarchy for running
multiple agents in parallel.

The **supervisor** (`clc up`) spawns coordinators based on `clc.yml`
topology configuration. Each **coordinator** scans for pickable tiskets
matching its selector (labels, project), dispatches **workers** into
isolated workspaces, handles permission requests, and lands completed
work.

Workers are individual agent sessions, each in its own workspace,
constrained by the phase system. When a worker needs a tool that isn't
pre-approved, it requests permission and stops. The coordinator can
grant, deny, or escalate to a human.

Permission policy is configurable: patterns can be auto-granted (safe
operations like `cargo test`), always-escalated (destructive operations
like `rm`), or left to the coordinator's judgment. The policy is part
of the coordinator's configuration.

See the [orchestration guide](/clc/orchestration) for the full workflow.

## What it's not

clc is not a CI/CD system. It doesn't build artifacts, manage
deployments, or interact with external services.

clc does not manage the agent's model, context window, or token budget.
It injects text and blocks tool calls — it doesn't control what the
agent thinks.

Missouri is the only test framework clc has built-in awareness of. The
phase system doesn't know about cargo test, pytest, or any other test
runner. An agent can run any test command, but the hooks don't parse
test output or auto-advance phases based on results. Phase transitions
are explicit commands.
