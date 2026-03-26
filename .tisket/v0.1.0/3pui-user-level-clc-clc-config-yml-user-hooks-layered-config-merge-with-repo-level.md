---
title: "user-level clc: ~/.clc/config.yml, user hooks, layered config merge with repo-level"
status: in_progress
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-26T23:14:19Z
updated: "2026-03-26T23:15:29Z"
---

## Problem

clc is entirely repo-scoped. Tools like almanac, tisket, zettel, and moose
have value outside any single repo — personal backlogs, cross-project
knowledge, skills that apply everywhere. Without user-level config, agents
in repos without `clc init` get no tool awareness at all: no almanac
skills listing, no tisket context, no zettel knowledge, no moose guidance.

The prime text injection only fires when a repo has been initialized.
Agents working in non-clc repos (or outside repos entirely) have no idea
these tools exist.

## Design

### Config layering

`~/.clc/config.yml` is the user-level config. When inside a repo with its
own `clc.yml`, both load and merge:

- **Skills**: union of user + repo sources. Built-in always present.
- **Tisket**: user-level points at a personal data repo (e.g., co.d).
  Repo-level `.tisket/` is the project backlog. Prime text shows both.
- **Zettel**: user-level always (knowledge is personal). Points at a
  data repo.
- **Phase/guards**: repo-level only. No workflow enforcement at user level.
- **Env**: repo overrides user on conflicts.
- **Tools** (moose, belmont): user-level config unless repo overrides.

### Hook registration

User-level hooks go in `~/.claude/settings.json` (managed by nix/home-manager).
Repo-level hooks go in `.claude/settings.local.json` (written by `clc init`).
Claude Code merges both — user hooks fire everywhere, repo hooks add
workflow enforcement.

Both hooks call `clc hook`. The handler:

1. Load `~/.clc/config.yml` — always
2. Load `./clc.yml` — if present
3. Merge and assemble prime text from both layers

### Prime text assembly

Without repo-level clc (any Claude Code session):
```
## Almanac (skills)
[built-in + user skills]

## Tisket
[personal: N open issues]

## Zettel
[N notes in knowledge base]

## Moose, Belmont
[tool awareness]
```

With repo-level clc (initialized project):
```
## Workflow (repo)
[phase enforcement, guards, branch rules]

## Tisket
[repo: N open issues | personal: N open issues]

## Almanac (skills)
[built-in + user + repo skills]

## Zettel, Moose, Belmont
[same as user level]
```

### Data storage

Tisket and zettel data lives in a git-backed repo — the personal config
repo (co.d). `~/.clc/config.yml` points at it:

```yaml
tisket:
  root: ~/Projects/co.d
zettel:
  root: ~/Projects/co.d
skills:
  - path: ~/Projects/co.d/skills
```

For nix/home-manager setups, `~/.clc/config.yml` is a managed file
declared in `home.nix`. The data directories inside co.d are mutable
state committed normally.

### `clc init --user`

Sets up user-level config. For nix users, prints what to add to
`home.nix`. For non-nix users, writes `~/.clc/config.yml` and adds
hooks to `~/.claude/settings.json`.

## Acceptance Criteria

- [ ] `clc hook` loads `~/.clc/config.yml` when present, falls back
      gracefully when absent
- [ ] User-level config merges with repo-level: skills union, tisket
      shows both, phase enforcement is repo-only
- [ ] Prime text includes almanac/tisket/zettel context in sessions
      without repo-level clc init
- [ ] `clc init --user` generates config and hook registration
- [ ] Existing repo-level behavior unchanged when no user config exists

## Done When

- `clc hook` in a non-initialized repo with `~/.clc/config.yml` injects
  almanac skills and tool awareness into the prime text
- `clc hook` in an initialized repo with both configs merges them correctly
- Missouri tests cover both scenarios
- `clc init --user` produces working config

## Scratch Notes
