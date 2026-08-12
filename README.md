# ✌️ codelikecody

Code like Cody (that's me). Opinionated workflow enforcement for coding agents.

codelikecody is a set of CLI tools. The tools enforce discipline on
autonomous coding agents. Three principles drive the design:

1. Verify agent work. Do not trust it.
2. Make undesired behavior impossible. Do not discourage it.
3. Compose the tools through text streams and the filesystem.

## Tools

Each tool is a standalone CLI. `clc` bundles the tools into one binary
(`clc tisket`, `clc missouri`, and so on) and adds workflow
orchestration. Each tool also runs on its own.

### 🔀 missouri

> Show-me state. Model-based testing where system behavior is represented
> as finite state automata.

missouri tests CLI tools. It models their behavior as directed graphs of
filesystem states. Each state is a real directory. Each transition is a
shell command. Each assertion is a byte-for-byte diff. The diff compares
the directory that a command produces against the directory you declared.
There is no assertion language and no mocking. The expected state is the
directory.

`missouri docs getting-started` · [missouri/](missouri/)

### 🧺 tisket

> 🎶 _A tisket, a tasket_ 🎶 A plaintext git-tracked CLI-first project
> management system for people that use coding agents.

tisket stores each issue as a markdown file with YAML frontmatter. The
files live in `.tisket/` in git. Each issue has a status, labels,
dependencies, and scratch notes for agent working memory. Agents read and
write issues as filesystem operations. tisket needs no external service
and no API token. It works offline.

`tisket docs getting-started` · [tisket/](tisket/)

### 🧛 belmont

> _What is a man? A miserable little pile of secrets._ Insecure
> 'best-effort' secret management for supplying credentials to LLM agents.

belmont resolves secrets from pluggable backends, such as the OS keychain
and environment variables. It injects the secrets into commands at
runtime. It also removes secret values from command output as the output
appears. Agents can then use credentials without reading them.

The threat model is narrow. belmont stops the most common LLM agent
exfiltration patterns. Two examples: an agent prints a `.env` file, or an
agent echoes an environment variable during troubleshooting. An agent that
attacks side channels can probably still get the secrets. Side channels
include subshell environment inspection and localhost echo servers.
**One developer wrote this codebase. I am not a security researcher. Do
not use belmont for anything security-critical. I do not use it in my own
professional work.**

`belmont --help` · [belmont/](belmont/)

### 📖 almanac

> An almanac (historically spelled almanack) is a regularly published
> listing of a set of current information about one or multiple subjects.
> —Wikipedia

almanac collects agent skills from local directories and built-in
sources. A skill is a directory with a SKILL.md file. The YAML
frontmatter declares the name and the description. The body holds the
instructions. almanac indexes every available skill and prints its
content on request. An agent then loads only the skills it needs for the
current task.

`almanac docs` · [almanac/](almanac/)

### 🗃️ zettel

> Zettelkasten for repos. Atomic notes, linked ideas, plain markdown
> in git.

zettel keeps a flat directory of notes in `.zettel/`. Each note has
frontmatter for tags, links, and status. zettel shows forward links and
backlinks, finds orphan notes, and walks the graph. Agents create draft
notes during research. People review the drafts and promote them.

`zettel docs getting-started` · [zettel/](zettel/)

### ✌️ clc (mothballed)

clc is the workflow engine that connected the other tools. It handled
tisket pickup, isolated workspaces, and supervisor, coordinator, and
worker orchestration. Its phase system also restricted edits at each TDD
stage. Development is paused, and this repo no longer runs under clc.
[gaff](https://github.com/cjohnhanson/gaff) now does the hook and
context-lifecycle work. A future version of clc would be a coding-agent
harness that composes with gaff instead of containing it.

`clc docs getting-started` · [clc/](clc/)

## Libraries

These internal crates hold concerns that the tools share.

- **[mdstore](mdstore/)** — Parses and serializes markdown documents
  with YAML frontmatter. It is the storage layer under tisket and
  zettel. It is generic over the frontmatter type. Any tool that stores
  structured data as markdown files in git can use it.

- **[clc-sdk](clc-sdk/)** — Defines traits for workspace lifecycle,
  agent tool integration, and coordination. The Workspace trait covers
  isolation backends such as git worktrees and Docker. The SDK also
  defines how a tool reports status and phase-aware directives to the
  workflow engine.

- **[claude-code](claude-code/)** — Defines protocol types for Claude
  Code's NDJSON streaming output. It parses assistant messages, tool
  use, results, and session metadata.

- **[clc-api](clc-api/)** — Wraps tisket's file-based issue repository
  in an Axum HTTP API. It provides REST endpoints to list, create, and
  edit issues.

- **[clc-web](clc-web/)** — Renders the issue tracker in the browser
  with Leptos. It provides a board view and an issue detail view.
  clc-api serves it as static files.

## Principles

**Show me, don't tell me.** An agent works at a high level of
abstraction. To trust that work, you must be able to check the output
quickly. So every output must be easy to verify.

**Undesired behavior should be impossible to perform.** If you do not
want something to happen, make it impossible. If you want something to
happen, make it impossible to skip.

**Text streams are the universal interface.** These tools apply the
[Unix philosophy](https://en.wikipedia.org/wiki/Unix_philosophy) to
agent tooling.

## Documentation

Each tool ships bundled docs. Read them with `<tool> docs [topic]`:

```
missouri docs writing-tests
tisket docs workflow
clc docs orchestration
almanac docs
zettel docs
```

Development conventions live in [AGENTS.md](AGENTS.md).
