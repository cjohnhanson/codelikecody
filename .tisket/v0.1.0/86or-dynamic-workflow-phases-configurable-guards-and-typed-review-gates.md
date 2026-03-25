---
title: "Dynamic workflow phases, configurable guards, and typed review gates"
status: todo
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-23T04:27:05Z
updated: "2026-03-25T02:58:51Z"
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
before the worker can advance. The coordination protocol already has
`ReviewRequest` and `ReviewResult` message kinds with a `ReviewVerdict`
enum, but nothing in the runtime uses them and they carry no review
type.

## Open Questions

### Dynamic phases and guards

- The guard currently receives `Option<Phase>` — a typed enum. Making
  this a string means the guard loses compile-time exhaustiveness
  checking. Is the trade-off acceptable, or should there be a validated
  "DynamicPhase" type that's checked against the workflow at construction?
- The guard needs the active workflow definition at evaluation time. The
  hook currently loads config from `cwd` (`config::load(cwd)`) but
  doesn't resolve which workflow applies — that happens at pickup. Should
  the active workflow name be stored in the state (DB or file) so the
  guard can look it up without re-resolving from tisket labels?
- `restricted_paths` in the current guard checks `path.contains("tests/missouri/")`.
  For configurable paths, should this be glob patterns? Prefix matching?
  Regex? Glob patterns match how Claude Code permissions work.
- The trunk Bash allowlist is a safety net against accidental writes on
  the main branch. Should it remain hardcoded (defense in depth) or
  become configurable (flexibility)? A hardcoded minimum plus
  configurable additions might be the right middle.
- The prime text currently describes the TDD workflow loop in prose.
  With dynamic workflows, the prime text needs to describe the *active*
  workflow's loop. Should this come from the workflow definition in
  config (a `description` or `instructions` field)? Or should the
  phase list be enough for the agent to infer the workflow?
- `done.rs` checks `current_phase != Phase::Done`. With dynamic phases,
  what's the terminal phase? The last phase in the workflow's list?
  An explicitly marked terminal phase? Always literally "done"?
- The worker system prompt (`dispatch.rs:379-392`) hardcodes the phase
  sequence. The worker prompt (`dispatch.rs:370-377`) hardcodes "follow
  the clc workflow: write tests, implement, get green, run `clc done`."
  Both need to become workflow-aware.
- `worker.rs:640` recover function hardcodes stepping through
  `review-requested → in-review → reviewed → done`. Recovery needs to
  understand the active workflow's phase sequence.
- `hook.rs:346-351` post-tool nudge only fires for `Phase::Implementing`.
  With dynamic phases, which phases get nudges? Should this be
  configurable per-workflow?
- `hook.rs:355-396` bootstrap logic hardcodes `tests-unwritten` as the
  initial phase for unphased feature branches. With dynamic workflows,
  bootstrap needs to resolve the workflow first.

### Typed review gates

- Who spawns the reviewer session? The worker calls `clc review request code`
  — does clc itself spawn a fresh `claude` process in the same worktree?
  Or does the coordinator handle it? If the worker spawns it, the worker
  is blocked (same worktree, can't make edits while reviewer reads). If
  the coordinator spawns it, there's coordination overhead.
- The reviewer runs in the same worktree — shared filesystem, but a
  fresh Claude session (no `--resume`). The reviewer's SessionStart hook
  needs to inject different context than the worker's. How does the hook
  know it's a reviewer session vs a worker session? A marker file in
  `.clc/`? An environment variable? A different `role` in the DB?
- Review prompts need to be configurable per review type in the workflow
  definition. A short `instructions` field in config is too limiting for
  real review guidance — "review for correctness" doesn't tell a reviewer
  agent what patterns to check, what the project's conventions are, or
  what past mistakes to watch for. Options:
  - `prompt` field in the review config: inline text, simple but limited
  - `prompt_file` pointing to a markdown file: richer, versionable, but
    adds a file to maintain
  - `skill` pointing to an almanac skill: richest — the reviewer loads
    a skill that teaches it how to do that type of review, same way
    workers load skills for development. Skills already exist for
    `code-review`, `security-review`, `writing-review` in this project's
    `skills/` directory
  - Some combination: short `instructions` for the hook injection, plus
    a `skill` for deep guidance the reviewer loads on its own
- Reviewer permissions need to be configurable per review type. The base
  is read-only tools plus the verdict commands (`clc review approve`,
  `clc review request-changes`). But different review types need
  different capabilities: a code reviewer might need `cargo test` and
  `cargo clippy`; a security reviewer might need `cargo audit` or
  pattern-grepping; a writing reviewer might need `agent-browser` to
  render output. The review config should declare `permissions.allow`
  and `permissions.deny` per type, seeded via `permissions::seed_defaults`
  the same way worker permissions are seeded. Example:
  ```yaml
  reviews:
    code:
      skill: code-review
      instructions: "Review for correctness and project patterns"
      permissions:
        allow: ["Bash(cargo test *)", "Bash(cargo clippy *)"]
        deny: ["Edit", "Write"]
      required_before: [done]
    security:
      skill: security-review
      instructions: "Review for injection, auth, secret exposure"
      permissions:
        allow: ["Bash(cargo audit *)"]
        deny: ["Edit", "Write"]
      required_before: [done]
  ```
- What happens if the reviewer crashes or times out without rendering a
  verdict? Is the review request left pending forever? Should there be
  a timeout after which the request can be re-issued?
- Can a worker request multiple review types simultaneously? If
  `code` review and `security` review are both required before `done`,
  can they run in parallel (two reviewer sessions in the same worktree)?
  Concurrent read-only sessions in the same worktree should be safe,
  but the guard needs to handle two verdict commands potentially racing.
- `request-changes` notifies the worker, but how? If the worker session
  is still alive (waiting on the review), it needs to receive the
  feedback and resume. Via the stdin pipe? Via a coordination DB message
  that the hook surfaces on next prompt? Via a phase regression that
  the hook announces?
- Should reviews be recorded permanently (in the coordination DB or
  tisket scratch notes) for audit? The review verdict and any comments
  are useful history.
- How does this interact with the existing review phases
  (`review-requested`, `in-review`, `reviewed`) in the hardcoded TDD
  workflow? Are those phases the *implementation* of review gates in the
  TDD workflow, or are they separate concepts?

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

The review skills already exist in `skills/` (`code-review`,
`security-review`, `writing-review`). The infrastructure for teaching
reviewer agents what to look for is already built. The review gate
mechanism is what connects "here's a skill that knows how to review"
to "a fresh agent loads that skill, gets appropriate permissions, and
renders a verdict that gates workflow progression."
