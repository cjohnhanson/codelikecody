---
title: "Dynamic workflow phases, configurable guards, and typed review gates"
status: in_progress
priority: 2
assignee:
labels: [clc, architecture, clc-up-target]
depends_on: []
created: 2026-03-23T04:27:05Z
updated: "2026-03-27T01:28:48Z"
---

## Problem

The workflow system has two layers that don't connect.

The config layer works: `clc.yml` accepts arbitrary workflow definitions
with custom phase lists, and rules that match issues to workflows by
label and project. `config::resolve_workflow` correctly selects the
matching workflow at pickup time.

The runtime layer is hardcoded. `Phase` is a Rust enum with exactly nine
TDD values. `phase::set()` and `phase::init_phase()` parse phase strings
through `Phase::from_str`, which rejects anything not in the enum. If
pickup resolves a workflow with `phases: [outline, draft, final]`, the
call to `init_phase(worktree, "outline")` fails with "unknown phase:
outline." The guard in `guard.rs` hardcodes which phases are unrestricted
(`Implementing | InReview`), restricts all others to edits in
`tests/missouri/`, and uses a const array for the trunk Bash allowlist.
The stop hook hardcodes which phases allow exit. The `done` ceremony
hardcodes `Phase::Done`. The prime text hardcodes the TDD workflow loop
description. The worker system prompt hardcodes the phase sequence.

The only workaround is a workflow whose sole phase is `done`, which
bypasses enforcement entirely. This means non-code work (documentation,
admin, research, writing) either gets no workflow structure at all, or
gets TDD structure that doesn't fit.

There is also no mechanism for review gates — points in the workflow
where a fresh, independent agent examines the work and must approve
before the worker can advance.

## Schema

### WorkflowDef

```yaml
workflows:
  <name>:
    description: <string>         # injected into prime text
    phases: [<PhaseDef>, ...]     # first is initial phase
    reviews:                      # optional
      <type>: <ReviewDef>
```

Workflows are selected by policy rules at pickup time (this already
works via `config::resolve_workflow`). The active workflow name is stored
in the supervisor DB so the guard can look it up without re-resolving.

### PhaseDef

```yaml
- name: <string>                    # kebab-case identifier
  instructions: <string>            # optional — injected into prime text
  nudge: <string>                   # optional — post-tool reminder
  can_stop: <bool>                  # default false
  permissions:                      # optional — absent = unrestricted
    allow: [<tool-pattern>, ...]
    deny: [<tool-pattern>, ...]
  transitions:                      # optional — absent = terminal phase
    - <string>                      # simple: target phase name
    - target: <string>              # rich: target + review gate
      requires: [<review-type>, ...]
```

Phases form a directed graph. Each phase declares its outgoing edges via
`transitions`. The runtime validates that target is in the current
phase's transitions list. No ordinal math, no forward/backward
distinction — just edge validation.

- **Initial phase**: first in the list (set on pickup by `init_phase`)
- **Terminal phases**: any phase with no `transitions` — `clc done`
  succeeds at any terminal phase
- **Self-transitions**: implicit — not transitioning means staying put
- **Permissions**: same syntax as Claude Code tool patterns. `deny`
  blocks the tool; `allow` with a glob punches exceptions. Absent
  permissions block = unrestricted.
- **YAML anchors**: use `&name` / `*name` for DRY when multiple phases
  share the same permission profile

### ReviewDef

```yaml
reviews:
  <type>:
    instructions: <string>        # injected into reviewer session prompt
    permissions:                  # reviewer's tool permissions
      allow: [<tool-pattern>, ...]
      deny: [<tool-pattern>, ...]
```

- Reviews are referenced by type name in transition `requires` lists
- `instructions` is a free-form string — can reference almanac skills
  ("load the `code-review-eval` skill"), inline short guidance, or both
- Verdict commands (`clc review approve`, `clc review request-changes`)
  are always available to reviewers, never to workers
- Reviewer identity verified via mTLS — worker cannot impersonate

### Tool patterns

Same syntax as Claude Code permissions:

- `"Edit"` — block/allow the tool entirely
- `"Edit(tests/**)"` — scoped to matching paths
- `"Bash(cargo test *)"` — scoped to matching commands

Evaluation order: deny first, then allow exceptions.

## Design Decisions

### Phases are a graph, not a sequence

The hardcoded linear sequence is an artifact of TDD being the only
workflow. Even TDD has backward edges (green → implementing on
regression). Other workflows need cycles (review → draft on changes
requested), shortcuts (triage → verify on false alarm), and multiple
terminals (done, abandoned).

Each phase declares outgoing edges. The runtime validates transitions
against those edges. The `Phase` enum is replaced with strings validated
against the active workflow definition.

### Review gates live on transitions, not phases

A gated transition uses `requires: [<review-type>, ...]` to declare
which review types must pass before the edge can be crossed. This is
an edge property, not a node property — different transitions out of
the same phase can have different review requirements (or none).

### Review lifecycle

1. Worker tries to cross a gated transition (`clc phase set done`)
2. Runtime sees `requires`, blocks the transition, dispatches review(s)
   via supervisor
3. Worker stops (stop hook allows exit when awaiting review)
4. Multiple required reviews run in parallel — concurrent read-only
   sessions in the same worktree
5. Verdicts land in supervisor
6. Supervisor resumes the worker
7. Worker's SessionStart hook delivers review results (approved, or
   changes-requested with feedback)
8. On approval: worker retries the transition, gate is now open
9. On changes-requested: gate stays closed, worker decides whether to
   regress or address feedback in place, then re-requests review

### Reviewer sessions

Fresh Claude session in the same worktree (no `--resume`). The
reviewer's SessionStart hook injects different context than the
worker's — the review instructions, appropriate permissions, and
verdict commands. The supervisor spawns and monitors the reviewer
lifecycle.

### Worker operational status

Separate from phase (which tracks the *work*), the supervisor tracks
worker operational state: running, stopped, awaiting-review,
awaiting-permission, done, failed. The supervisor resume loop checks
this status rather than sniffing for indirect signals. This is a
runtime concern, not part of the workflow schema.

### Trunk allowlist stays project-level

The Bash allowlist on the main branch is defense-in-depth independent
of any workflow. It stays hardcoded / project-config, not per-workflow.

## Examples

### TDD workflow

```yaml
workflows:
  tdd:
    description: >
      Test-driven development. Write failing tests, implement until
      green, request code review, finalize.

    phases:
      - name: tests-unwritten
        instructions: "Write failing tests that specify the desired behavior."
        transitions: [tests-written]
        permissions: &test-only
          allow: ["Edit(tests/**)", "Write(tests/**)", "Bash(cargo test *)"]
          deny: ["Edit", "Write", "Bash"]

      - name: tests-written
        instructions: "Verify tests fail for the right reasons."
        transitions: [red, tests-unwritten]
        permissions: *test-only

      - name: red
        instructions: "Tests are red. Confirm failures match expectations."
        transitions: [implementing, tests-unwritten]
        permissions: *test-only

      - name: implementing
        instructions: "Write the minimum code to make failing tests pass."
        nudge: "Run tests to check your progress."
        transitions: [green, red]

      - name: green
        instructions: "Tests pass. Refactor if needed."
        can_stop: true
        transitions:
          - implementing
          - target: done
            requires: [code]

      - name: done

    reviews:
      code:
        instructions: >
          Load the code-review-eval skill. Review for correctness,
          project patterns, and test coverage.
        permissions:
          allow: ["Bash(cargo test *)", "Bash(cargo clippy *)"]
          deny: ["Edit", "Write", "Bash"]
```

### Docs workflow

```yaml
workflows:
  docs:
    description: >
      Documentation writing. Outline, draft, review, finalize.

    phases:
      - name: outline
        instructions: "Establish document structure and section plan."
        transitions: [draft]
        permissions: &docs-edit
          allow: ["Edit(docs/**)", "Write(docs/**)"]
          deny: ["Edit", "Write"]

      - name: draft
        instructions: "Write sections per the outline. Source-verify technical claims."
        transitions: [review, outline]
        permissions: *docs-edit

      - name: review
        instructions: "Documentation is under review."
        can_stop: true
        transitions:
          - draft
          - target: done
            requires: [writing]

      - name: done

    reviews:
      writing:
        instructions: >
          Load the writing-review skill. Review for clarity, accuracy,
          and Diataxis type discipline.
        permissions:
          deny: ["Edit", "Write", "Bash"]
```

### Admin workflow (minimal)

```yaml
workflows:
  admin:
    description: "Administrative work — triage, planning, cleanup."
    phases:
      - name: working
        instructions: "Do the work."
        can_stop: true
        transitions: [done]
      - name: done
```

## Runtime Changes Required

These are the hardcoded sites that need to become workflow-aware:

- `phase.rs`: `Phase` enum → string validated against active workflow's
  phase list. `set()` validates transitions against the phase's
  `transitions` list instead of ordinal math.
- `guard.rs`: loads the active workflow and current phase's `PhaseDef`,
  applies its `permissions` block instead of hardcoded unrestricted/
  restricted logic. Trunk allowlist stays separate.
- `hook.rs` (stop): checks `can_stop` on the current `PhaseDef` instead
  of hardcoded phase match.
- `hook.rs` (post-tool nudge): checks `nudge` field on current
  `PhaseDef` instead of hardcoded `Phase::Implementing` check.
- `hook.rs` (bootstrap): resolves workflow first, uses `phases[0]`
  instead of hardcoding `tests-unwritten`.
- `done.rs`: checks whether current phase has no `transitions` (terminal)
  instead of checking `Phase::Done`.
- `dispatch.rs` (worker prompt): injects workflow `description` and
  current phase `instructions` instead of hardcoded TDD description.
- `worker.rs` (recover): reads the workflow's phase graph instead of
  hardcoding `review-requested → in-review → reviewed → done`.
- Prime text: injects workflow `description` and phase `instructions`
  from config.

## Why It Matters

Without this, clc is a TDD enforcement engine that can only do one
thing. The tools underneath (tisket, missouri, zettel, coordination DB)
are general-purpose, but the workflow layer forces everything through a
nine-phase code development funnel. Non-code work (writing, research,
triage, operations) either bypasses the workflow entirely (`clc admin`,
or a `done`-only workflow) or doesn't happen in clc at all.

Typed review gates are the mechanism that makes autonomous work
trustworthy. Without them, the only check on worker output is "did the
tests pass." For non-code work there may not be tests. For code work,
passing tests is necessary but not sufficient — code review catches
things tests don't. A reviewer agent with constrained permissions and
fresh context is the automated equivalent of a code review, and gating
phase advancement on review approval is how you enforce it.

## Scratch Notes

### Done — 18 commits on branch 86or

**Schema + engine:**
- PhaseDef, TransitionDef, PermissionsDef, ReviewDef in config.rs
- Workflow engine (workflow.rs) — directed graph validation, default_tdd()
- Phase enum deleted — all consumers use string phases + Workflow graph

**Guard + hook:**
- Guard uses permission patterns (deny/allow with tool globs)
- Hook: prime text, nudge, bootstrap, stop all workflow-driven
- Reviewer session detection via CLC_REVIEW_TYPE env var
- Reviewer prime text: review instructions, verdict commands, tisket context

**Review gates:**
- review.rs: CLI commands (request/approve/request-changes)
- review_type field on ReviewRequest/ReviewResult messages
- Transition gating: set_with_workflow checks DB for required approvals
- Coordinator spawns reviewer sessions for pending reviews
- Worker supervise blocks on pending reviews

**Identity:**
- Bearer token per agent at registration (stored in coordination_agents.token)
- Client sends Authorization: Bearer header via CLC_AGENT_TOKEN env var
- Supervisor validates token identity on ReviewResult messages

**Consumer migration:**
- done.rs, worker.rs, dispatch.rs, merge.rs, pickup.rs, main.rs all use Workflow
- Recover walks forward graph edges (no hardcoded phase sequence)

**Verification:**
- 189 clc unit tests, zero failures
- Full workspace: zero warnings, zero failures
- Missouri: 37 paths, 106 steps, 4562 assertions, zero failures
- New missouri states: has-custom-workflow-config, picked-up-custom-workflow

### Follow-up
- p7yy (mTLS cert extraction) can be cancelled — bearer token auth covers it

### Files modified
- clc/src: config.rs, workflow.rs (new), phase.rs, guard.rs, hook.rs,
  done.rs, worker.rs, dispatch.rs, merge.rs, pickup.rs, main.rs, cli.rs,
  review.rs (new), supervisor_api.rs, coordination.rs, coordination_client.rs
- clc-sdk/src: coordination.rs, coordination_db.rs
- clc-sdk/examples: coordination_exercise.rs
- clc/tests/missouri: initialized, has-custom-workflow-config (new),
  picked-up-custom-workflow (new), ready-to-done, stranded-at-green,
  has-custom-workflow-config (new)
- moose/src/native: e2e_tests.rs (new), parity_tests.rs (new)
