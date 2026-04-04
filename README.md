# ✌️ codelikecody

Code like Cody (that's me). Opinionated workflow enforcement for coding agents.

codelikecody is a set of CLI tools that enforce discipline on autonomous
coding agents. Three principles drive the design: agent work should be
verifiable, not trusted; undesired behavior should be mechanically
impossible, not merely discouraged; and the tools compose through text
streams and the filesystem.

## Tools

Each tool is a standalone CLI. `clc` bundles them into a single binary
(`clc tisket`, `clc missouri`, etc.) and adds workflow orchestration
on top, but they work independently.

### 🔀 missouri

> Show-me state. Model-based testing where system behavior is represented
> as finite state automata.

Tests CLI tools by modeling their behavior as directed graphs of filesystem
states. Each state is a real directory. Transitions are shell commands.
Assertions are byte-for-byte diffs between the directory after a command
runs and the directory you said it should produce. No assertion DSL, no
mocking. The expected state is the directory.

`missouri docs getting-started` · [missouri/](missouri/)

### 🧺 tisket

> 🎶 _A tisket, a tasket_ 🎶 A plaintext git-tracked CLI-first project
> management system for people that use coding agents.

Issues are markdown files with YAML frontmatter, stored in `.tisket/` in
git. Status lifecycle, labels, dependencies, per-issue scratch notes for
agent working memory. Agents read and write issues as filesystem
operations. No external service, no API tokens, works offline.

`tisket docs getting-started` · [tisket/](tisket/)

### 🧛 belmont

> _What is a man? A miserable little pile of secrets._ Insecure
> 'best-effort' secret management for supplying credentials to LLM agents.

Resolves secrets from pluggable backends (OS keychain, environment
variables) and injects them into commands at runtime. Scrubs secret values
from command output in real time so agents can use credentials without
seeing them.

The threat model is narrow: prevent the most common LLM agent exfiltration
patterns, like an agent cat'ing a `.env` file or echoing an environment
variable while troubleshooting. An agent that actively tries to extract
secrets through side channels (subshell env inspection, localhost echo
servers) can probably succeed. This is a solo-developed codebase. I am
not a security researcher. Do not use this for anything security-critical.
I do not use this in my own professional work.

`belmont --help` · [belmont/](belmont/)

### 📖 almanac

> An almanac (historically spelled almanack) is a regularly published
> listing of a set of current information about one or multiple subjects.
> —Wikipedia

Aggregates agent skills from local directories and built-in sources.
A skill is a directory with a SKILL.md file: YAML frontmatter declares
name and description, the body holds instructions. Almanac indexes
all available skills and prints their content on demand, so agents
load only the skills relevant to the current task.

`almanac docs` · [almanac/](almanac/)

### 🗃️ zettel

> Zettelkasten for repos. Atomic notes, linked ideas, plain markdown
> in git.

A flat directory of notes in `.zettel/`, each with frontmatter for tags,
links, and status. Forward links, backlinks, orphan detection, graph
traversal. Agents create draft notes during research; humans review
and promote them.

`zettel docs getting-started` · [zettel/](zettel/)

### 🫎 moose

> Predominantly a browser, the moose's diet consists of both terrestrial
> and aquatic vegetation, depending on the season, with branches, twigs
> and dead wood making up a large portion of their winter diet. —Wikipedia

Browser automation for AI agents. Forked from
[agent-browser](https://github.com/vercel-labs/agent-browser). Connects
to Chrome via CDP, handles screenshots, form filling, navigation, and
session recording.

`moose --help` · [moose/](moose/)

### ✌️ clc

The workflow engine that ties everything together. Picks up tisket issues,
creates isolated workspaces, enforces a phase system that gates what agents
can edit at each stage of test-driven development, and optionally
orchestrates multiple agents working in parallel with a
supervisor/coordinator/worker hierarchy. Workspace isolation is
pluggable: git worktrees and Docker containers are the current
backends.

Workflows are configurable. The default is a TDD sequence, but any
phase graph can be defined in `clc.yml`, with per-phase permissions,
review gates, and policy rules that select workflows based on issue
labels or project. A guard intercepts every tool call and rejects
disallowed operations before they reach the filesystem.

`clc docs getting-started` · [clc/](clc/)

## Libraries

Internal crates that factor out shared concerns.

- **[mdstore](mdstore/)** — Parses and serializes YAML-frontmatter
  markdown documents. The storage layer underneath tisket and zettel.
  Generic over frontmatter type, so any tool that stores structured
  data as markdown files in git can use it.

- **[clc-sdk](clc-sdk/)** — Traits for workspace lifecycle, agent
  tool integration, and coordination. The Workspace trait abstracts
  over isolation backends (git worktrees, Docker, others). Defines
  how tools report status and phase-aware directives to the workflow
  engine.

- **[claude-code](claude-code/)** — Protocol types for Claude Code's
  NDJSON streaming output. Deserializes assistant messages, tool use,
  results, and session metadata.

- **[clc-api](clc-api/)** — Axum HTTP API wrapping tisket's file-based
  issue repository. REST endpoints for listing, creating, and editing
  issues.

- **[clc-web](clc-web/)** — Leptos client-side rendered frontend for
  the issue tracker. Board view and issue detail, served as static
  files by clc-api.

## Principles

**Show me, don't tell me.** The proof of the pudding is in the eating.
To be able to work autonomously at a high level of abstraction, agent
outputs need to be easily verifiable.

**Undesired behavior should be impossible to perform.** If you don't
want something to happen, make it impossible. If you want something
to happen, make it impossible not to happen.

**Text streams are the universal interface.** [Unix philosophy](https://en.wikipedia.org/wiki/Unix_philosophy)
applied to agent tooling.

## Documentation

Each tool ships bundled docs accessible via `<tool> docs [topic]`:

```
missouri docs writing-tests
tisket docs workflow
clc docs orchestration
almanac docs
zettel docs
```

Development conventions live in [AGENTS.md](AGENTS.md).
