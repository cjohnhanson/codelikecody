---
title: "Restructure clc prime text as skills with progressive disclosure"
status: todo
priority: 2
assignee:
labels: [clc, ergonomics]
depends_on: []
created: "2026-03-05T04:15:00Z"
updated: "2026-03-05T04:15:00Z"
---

## Problem

The current prime text is a ~150-line monolithic blob injected on SessionStart. Every session gets everything: workflow loop, TDD mandate, working memory instructions, commit discipline, tisket usage, missouri usage — regardless of whether any of it is relevant to what the agent is actually doing.

This is the opposite of the skill pattern that Anthropic recommends. Skills use progressive disclosure: description-only in ambient context, full content loaded on invocation, supporting files deferred.

## What the prime is actually doing

The prime text is a collection of skills:
- **Workflow engine** — what clc is, what hooks enforce, trunk vs worktree rules
- **Phase system** — the TDD loop, phase transitions, what's allowed when
- **Working memory** — scratch notes in tisket files, persistence across compaction
- **Commit discipline** — stage frequently, pre-commit hooks, clean tree for done
- **Discovered work** — create tiskets for tangents instead of pursuing them
- **Tisket** — issue tracker context (dynamic: open count, active issue, status)
- **Missouri** — test runner context (dynamic: pass/fail counts)

Each of these is conceptually a skill. The agent needs the workflow engine awareness always, but the TDD methodology details only matter during implementation. The scratch notes pattern only matters when context is getting long. Missouri details only matter when running tests.

## Proposed structure

### Ambient context (always loaded, kept minimal)
What clc is. That hooks govern this session. Current state (branch, phase, active tisket). How to get more detail.

Like a skill description — enough to know it exists and when to invoke it, not the full content.

### On-demand sections (loaded when relevant)
Each section becomes its own prime fragment, loaded when the phase or context makes it relevant:
- Phase enters `tests-unwritten` → TDD methodology loads
- Phase enters `implementing` → commit discipline loads  
- Agent is on trunk → trunk rules load (already conditional)
- Context compaction detected → working memory guidance loads
- Missouri tests exist → missouri runner guidance loads

### Progressive depth
`clc prime` prints the full assembled text (debugging). But each section should also be individually retrievable: `clc prime --section tdd`, `clc prime --section working-memory`. The hook system selectively injects based on phase/context rather than dumping everything.

### Follow Anthropic's skill conventions
- Keep ambient context under ~30 lines (like a skill description)
- Reference deeper content explicitly: "For TDD methodology details, the phase system will inject guidance when you enter test-writing phases"
- Dynamic context injection (`!command` equivalent) already works via the hook system
- Supporting material deferred to phase-appropriate moments

## What this is NOT

Not converting clc into Claude Code skills (`.claude/skills/`). The hook-based injection system is the right mechanism — it's more powerful than skills because it's event-driven and phase-aware. But the *authoring patterns* from skills (progressive disclosure, minimal ambient footprint, deferred detail) should inform how the prime text is structured.

## Verification

- SessionStart context is under 40 lines (currently ~150)
- Phase transitions inject relevant methodology
- Agents still follow TDD, commit discipline, etc. (behavioral regression test)
- `clc prime` still shows the full assembled text for debugging

## Key principle: navigable tree, not selective omission

The goal is NOT "show less." Every detail about how to use clc, tisket, missouri, phases, etc. should be accessible — just organized as a tree of progressive disclosures rather than a flat dump.

The ambient context is the root node. It names every concept and provides a path to its full detail. Each concept branches into its operational specifics. Those specifics can reference deeper implementation details.

```
root (ambient, ~30 lines)
├── workflow engine
│   ├── hook system (what fires when)
│   ├── trunk rules (read-only, allowlist)
│   └── worktree rules (phase-gated writes)
├── phase system
│   ├── TDD methodology (write test → red → green → refactor)
│   ├── phase transitions (what advances when)
│   └── guard behavior (what's blocked per phase)
├── working memory
│   ├── scratch notes format
│   └── compaction survival strategy
├── commit discipline
│   ├── pre-commit hooks
│   └── staging patterns
├── tisket (issue tracker)
│   ├── CLI reference
│   └── issue lifecycle
└── missouri (test runner)
    ├── state graph model
    └── running tests
```

The mechanism for traversal is `clc prime --section <path>` at the CLI level, and phase-triggered injection at the hook level. But the tree is always complete — nothing is hidden, just deferred until the agent reaches for it or the context makes it relevant.

This is the same pattern as Anthropic's skill system: description → full content → supporting files. Applied recursively.

## Untracked mode: stealth constraint

When a project is initialized with `clc init --untracked`, the `.clc/` directory is excluded from git via `.git/info/exclude`. The agent's workflow tooling is invisible to the project's version control.

The prime must communicate this constraint:
- Do not mention clc, tisket, missouri, phases, or any workflow tooling in commit messages, PR descriptions, documentation, or any content that enters the project's git history
- Commit messages should read as if a human wrote them without tooling assistance
- The `is_untracked` flag is already available via `.clc/state` — prime assembly should check it and inject the stealth constraint when true

This is not about the tree restructure per se, but it's a prime content gap that should be addressed in the same pass.
