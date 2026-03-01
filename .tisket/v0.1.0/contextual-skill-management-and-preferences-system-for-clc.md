---
title: "Contextual skill management and preferences system for clc"
status: discovery
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T20:25:00Z
updated: "2026-03-01T16:52:21Z"
---

## Problem

Agent configuration (preferences, skills, behavioral directives) currently lives in a flat global CLAUDE.md file that contains everything for every project. The file has project-specific sections (picnic workflow, lodicule patterns, etc.) that are always loaded regardless of which project is active. This is:

- **Wasteful**: context window stuffed with irrelevant instructions
- **Fragile**: one big file with unrelated concerns
- **Not contextual**: no awareness of which project, worktree, or phase is active

clc already does contextual injection via hooks (prime text, reinforcement, post-tool nudges). The same system could manage preferences and skills contextually.

## Two feature areas

### 1. Preferences

User preferences for how the agent should behave. Currently in global CLAUDE.md:
- Tone and voice rules
- Development workflow (TDD, commit discipline)
- Project-specific conventions (picnic uses worktrees under .picnic-worktrees/, lodicule uses specific patterns)
- Tool preferences (always use uv, never use hasattr, etc.)

What contextual preferences could look like:
- Global preferences that always apply (tone, voice, general dev practices)
- Project-level preferences activated when working in that project
- Phase-level preferences (e.g., different focus during tests-unwritten vs implementing)
- Injected into prime text by clc, not loaded via flat CLAUDE.md

### 2. Skills

Claude Code skills (slash commands, specialized workflows). Currently defined per-project or globally. clc could:
- Register skills contextually based on project and phase
- Provide clc-specific skills (e.g., /pickup, /done, /status)
- Manage skill availability based on workflow state

## Discovery needed

This is a broad design space. Before implementation:

- How does Claude Code's skill system work? What's the registration mechanism?
- How does CLAUDE.md loading work? Can clc intercept or replace it, or does it need to work alongside?
- What's the right granularity for preferences? Per-project? Per-directory? Per-phase?
- How should project-specific preferences be stored? In .clc/? In the tisket? In a separate config?
- What existing patterns in other tools (direnv, editorconfig, etc.) are worth learning from?
- How does this interact with the hook consolidation tisket? Are preferences just another form of contextual hook behavior?

## Relationship to other tiskets

- **consolidate-external-shell-hooks-into-clc-hook-system**: hooks are the enforcement mechanism; preferences are the configuration layer above them
- **prime-text-operational-instructions**: prime text is the delivery mechanism for contextual preferences today
