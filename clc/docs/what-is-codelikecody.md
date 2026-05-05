<!-- metadata
title: "What is codelikecody?"
description: "Philosophy, tools, and how they fit together"
type: explanation
-->

# What is codelikecody?

Codelikecody is a workflow engine for coding agents. Hooks intercept
every tool call in an agent's session and enforce constraints based
on the current workflow phase. Agents work in isolated workspaces
(git worktrees or Docker containers) and can't modify trunk directly.

Seven tools, five supporting libraries. Each tool has its own binary
and works independently.

## The tools

### clc — workflow enforcement

clc runs as a hook system inside Claude Code, intercepting every event
in the agent's session: tool calls, prompt submissions, stop attempts.
Based on the current git branch and workflow phase, it decides what's
allowed and what's blocked. See the [CLI reference](/clc/cli-reference).

Workflows define a directed graph of phases, each with permissions (what
files can be edited), instructions (injected into agent context), and
stop gates (whether the agent can exit). The default workflow is a TDD
sequence:

`tests-unwritten` → `tests-written` → `red` → `implementing` → `green`
→ `review-requested` → `in-review` → `reviewed` → `done`

Custom workflows can define any phase graph. Policy rules in `clc.yml`
select workflows based on issue labels or project.

A guard intercepts every tool call and evaluates it against the current
phase's permissions. If the permission set says only test files can be
edited, edits to source files are rejected before they reach the
filesystem.

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

## How they fit together

Tisket holds work as issues. clc picks up an issue, creates a workspace,
and sets the initial workflow phase. Missouri tests run inside the
workspace to verify behavior. When the workflow completes, `clc done`
closes the tisket and cleans up. Almanac, belmont, and zettel fill
supporting roles: skills, secrets, and notes.

clc detects which tools are present in the project directory and injects
their status into the agent's context at session start and on every
prompt.

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

Context decays over long sessions. The original task, current phase,
and test status can drift out of effective context as the conversation
grows. clc re-injects orientation on every prompt submission and
nudges after file edits.

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
