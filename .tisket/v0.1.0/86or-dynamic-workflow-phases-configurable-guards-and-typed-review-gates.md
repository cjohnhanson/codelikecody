---
title: "Dynamic workflow phases, configurable guards, and typed review gates"
status: discovery
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: "2026-03-23T04:27:05Z"
updated: "2026-03-23T04:27:05Z"
---

## Problem

The workflow system has two layers that don't connect:

1. **Config layer** (works): `clc.yml` accepts arbitrary workflow definitions
   with custom phase lists, and rules that match issues to workflows by
   label/project. Selection logic in `config::resolve_workflow` is correct.

2. **Runtime layer** (hardcoded): `Phase` is a fixed enum with 9 TDD values.
   `phase::set()` and `phase::init_phase()` reject any string not in the
   enum. The guard in `guard.rs` hardcodes which phases are unrestricted
   (`Implementing | InReview`) and restricts all others to `tests/missouri/`
   paths. The Bash allowlist is a const array of cargo/git commands.

Result: you can define a `writing` workflow with `[outline, draft, review, final]`
in config, but pickup fails with "unknown phase: outline". The only escape
hatch is a workflow whose sole phase is `done`, which skips enforcement entirely.

This blocks using clc for non-code work (documentation, admin, research, writing)
and prevents meaningful review gates.

## What done looks like

### 1. Dynamic phases

- Phase becomes a string, not an enum. Any phase name is valid if it appears
  in the active workflow definition.
- Phase transitions are validated against the workflow's phase list (ordering,
  forward/backward rules) rather than a hardcoded ordinal.
- `clc status set <phase>` works with whatever phases the workflow defines.

### 2. Configurable guards

Each workflow definition declares its own guard policy:

```yaml
workflows:
  tdd:
    phases: [tests-unwritten, tests-written, red, implementing, green, done]
    unrestricted_phases: [implementing]
    restricted_paths: ["tests/missouri/"]
    exit_phases: [done, review-requested, reviewed]
    bash_allowlist: ["cargo *", "git *", "clc *"]
  writing:
    phases: [outline, draft, review, final]
    unrestricted_phases: [outline, draft]
    restricted_paths: ["docs/"]
    exit_phases: [final]
    bash_allowlist: ["git *", "clc *", "agent-browser *"]
```

- `unrestricted_phases`: all edits allowed (currently hardcoded to Implementing/InReview)
- `restricted_paths`: what file paths are allowed in non-unrestricted phases
  (currently hardcoded to `tests/missouri/`)
- `exit_phases`: which phases allow the agent to stop (currently hardcoded)
- `bash_allowlist`: per-workflow Bash command prefixes (currently a const array)

The guard reads the active workflow from config + current phase, and evaluates
dynamically instead of pattern-matching on enum variants.

Trunk guard is separate — its allowlist might also become configurable at the
project level rather than per-workflow.

### 3. Typed review gates

Workers can request reviews of a specific type. Reviews are performed by a
fresh agent session in the same worktree (shared filesystem, isolated context).

**Config:**

```yaml
reviews:
  code:
    required_before: [done]
    instructions: "Review for correctness, test coverage, and adherence to patterns"
  security:
    required_before: [done]
    instructions: "Review for OWASP top 10, injection risks, auth/authz issues"
  writing:
    required_before: [final]
    instructions: "Review for clarity, accuracy, and audience fit"
```

**Worker flow:**
1. Worker calls `clc review request <type>` (e.g., `clc review request code`)
2. Request recorded in `.clc/` or coordination DB
3. Phase advancement to any phase listed in `required_before` is blocked
   until review resolves

**Reviewer flow:**
1. Fresh Claude session launched in same worktree — no `--resume`, clean context
2. SessionStart hook injects review-type-specific instructions and constraints
3. Reviewer guard is extremely scoped: read-only tools + exactly two commands:
   - `clc review approve <type>`
   - `clc review request-changes <type> "<reason>"`
4. No file edits, no Bash beyond the verdict command

**Resolution:**
- `approve`: gate clears, worker can advance past the gated phase
- `request-changes`: worker is notified (via coordination DB message or
  `.clc/` file), reason is surfaced. Worker may need to regress phase
  (e.g., back to implementing) and re-request review after changes.

**Multiple reviews:** A phase can require multiple review types. All must
approve before advancement. Reviews are independent — requesting one doesn't
block requesting another.

## Open questions

- Should the trunk Bash allowlist also be configurable in `clc.yml`, or is
  that always a hardcoded safety net?
- Review instructions are prompt content — how much should live in config
  vs. in skill files the reviewer session loads?
- Should review approve/request-changes be idempotent? (Probably yes.)
- Can a coordinator trigger reviews automatically at certain phase
  transitions, or is it always worker-initiated?
- How does this interact with the existing `required_attempts` mechanism?
  Are attempts per-phase or per-workflow?

## Scope notes

This is probably multiple tiskets worth of work. The three pieces (dynamic
phases, configurable guards, typed reviews) have a dependency chain but
could land incrementally:

1. Dynamic phases + configurable guards (unblocks non-code workflows)
2. Review request/approve protocol (adds review gates)
3. Reviewer session orchestration (automates review execution)
