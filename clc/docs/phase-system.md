<!-- metadata
title: "The Phase System"
description: "How clc enforces workflows through configurable phase graphs"
type: explanation
-->

# The Phase System

The phase system constrains which files an agent can edit and whether it's
allowed to stop working. Workflows define a directed graph of phases, each
with its own permissions, instructions, and stop gates. The constraints are
enforced by hooks that intercept every tool call in a session — the agent
can't opt out.

Workflows are configurable. The default is a TDD sequence (described
below), but any phase graph can be defined in `clc.yml`. Policy rules
select workflows based on issue labels or project. See the
[configuration reference](/clc/config-reference) for the full schema.

## Why phases exist

Without constraints, agents skip tests and declare work done prematurely.
Phases make it mechanically impossible to edit source files before tests
exist, and mechanically impossible to stop before the workflow reaches a
designated exit point.

## The default TDD workflow

The built-in workflow defines nine phases as an ordered sequence. This is
what clc uses when no custom workflow is configured:

1. **tests-unwritten** — Work has been picked up but no tests exist yet.
   This is the starting state after `clc pickup`.

2. **tests-written** — Test files have been created or modified. The tests
   should define the desired behavior but are not expected to pass yet.

3. **red** — Tests have been run and they fail. This is the confirmation
   that the tests are actually testing something — a test that passes
   before any implementation is written is likely not testing the right
   thing.

4. **implementing** — The agent is writing production code to make the
   tests pass. This is the only phase (along with in-review) where
   unrestricted file edits are allowed.

5. **green** — Tests pass. Implementation is complete. But the work isn't
   done — it still needs to go through review.

6. **review-requested** — The work has been submitted for review. A PR
   exists or review has been otherwise requested.

7. **in-review** — Review is actively happening. The reviewer (or an agent
   responding to review feedback) may need to make changes, so file edits
   are unrestricted in this phase, same as implementing.

8. **reviewed** — Review is complete. Changes have been approved.

9. **done** — Work is finalized.

## Phase permissions

Each phase can define `permissions.allow` and `permissions.deny` patterns
that control which tool calls are permitted. In the default TDD workflow,
the guard splits phases into two categories:

**Unrestricted phases** (implementing, in-review): all tool calls pass
through. The agent can edit any file, run any command.

**Restricted phases** (everything else): file-targeting tools — Edit,
Write, NotebookEdit — are blocked unless the target path is inside
`tests/missouri/`. This forces test-first development: in the early
phases, the only files you can touch are test files.

Custom workflows can define any permission set per phase. A phase could
allow edits only to documentation files, or only to a specific directory,
or deny a specific tool entirely. Patterns use tool names with optional
glob arguments: `"Edit(tests/**)"`, `"Bash(cargo *)"`, `"Write"`.

The evaluation order: deny patterns are checked first. If a tool call
matches a deny pattern, it's blocked. If the phase has allow patterns
and the tool call doesn't match any, it's blocked. If no permissions
are defined for the phase, all tools pass.

Bash commands, read-only tools (Read, Glob, Grep), and other
non-file-targeting tools are allowed in all phases by the default
workflow. Custom workflows can restrict these too via explicit deny
patterns.

## The stop guard

The stop hook prevents the agent from ending its session prematurely. On
a feature branch, the agent is only allowed to stop in three phases:

- **review-requested** — work is submitted, waiting on someone else
- **reviewed** — review is done, work is approved
- **done** — everything is finalized

Every other phase blocks the stop event. **green blocks stop** because
tests passing doesn't mean work is reviewed and ready to merge.

When stop is blocked, the agent receives a message telling it the current
phase and suggesting `clc done` to finalize. The agent can't just ignore
this and stop anyway; the hook returns a non-zero exit code that prevents
the stop from completing.

The stop guard does not apply on the main branch or the admin branch.
Those contexts have different workflows where phase enforcement doesn't
make sense.

## Phase transitions

Transitions follow two rules:

**Forward by exactly one step.** An agent at red can move to implementing,
but not directly to green. Skipping phases is rejected. This prevents an
agent from jumping straight from tests-written to green without actually
going through the red-green cycle.

**Backward to any earlier phase.** An agent at reviewed can jump back to
implementing if review feedback requires changes. There's no restriction
on how far back — sometimes review reveals that the tests themselves need
rethinking, which means going all the way back to tests-written or even
tests-unwritten.

Attempting to set the current phase (a no-op transition) is also rejected.
If you're already at implementing, `clc status set implementing` fails
with "already at phase 'implementing'."

When no phase has been set, the only valid target is tests-unwritten. The
system won't let work start in the middle.

## The attempt gate

Forward transitions can optionally require multiple attempts before
succeeding. This is controlled by the `required_attempts` configuration
value (default: 1, meaning transitions succeed immediately).

When `required_attempts` is greater than 1, the first N-1 attempts to
advance a phase are rejected. Each rejected attempt increments a counter
stored alongside the phase in `.clc/state`. On the Nth attempt, the
transition succeeds and the counter resets to zero.

The purpose is to force a pause before phase transitions. When an agent
decides tests are done and wants to move to implementing, the attempt
gate makes it reconsider at least once. The rejection message says
"reconsider before trying again" — the idea is that the delay creates
a moment for the agent to actually look at what it's done rather than
rushing through phases.

The attempt counter only applies to forward transitions from an existing
phase. Backward transitions and the initial transition to tests-unwritten
are not gated.

## Trunk protection

Trunk protection is separate from the phase system but enforced by the
same guard infrastructure. On the main branch, the guard blocks all
file-modifying tools (Edit, Write, NotebookEdit) unconditionally. Bash
commands are restricted to an allowlist of read-only operations — git,
cargo test/check/build/clippy/fmt, clc, missouri, tisket queries, and
basic filesystem inspection tools like ls and cat.

The intent is that trunk is read-only. All work happens in worktrees
created by `clc pickup`. Trunk protection doesn't involve phases at all;
it's a blanket restriction based on branch detection.

The admin branch (`clc-admin` by default) is fully permissive — no phase
enforcement, no file restrictions, no stop guard. Admin work (triage,
planning, configuration changes) operates outside the TDD workflow.

## The guard system

All of this enforcement happens in the guard module, which evaluates
every event the hook receives. The hook reads JSON from stdin (provided
by the agent framework), detects the current git state and phase, and
passes the event to the guard.

The guard returns one of three responses:

- **Passthrough** — no opinion, let the tool call proceed
- **Allow with context** — let it proceed but inject a message (used for
  session start and prompt reinforcement, not by the guard itself)
- **Block with message** — reject the tool call and feed the message back
  to the agent

For PreToolUse events, the guard checks the tool name and input against
the current branch and phase constraints. For Stop events, it checks
whether the current phase permits exit. All other event types pass through.

There's an escape hatch: setting `CLC_GUARD_OFF=1` in the environment
disables all guard checks. This exists for developing clc itself, when
the guard code is being modified and you don't want the in-progress guard
to interfere with its own development.

## Phase bootstrap

Worktrees created outside of `clc pickup` — via plain `git worktree add`, for example — start with no phase set. Without intervention, this creates a dead state: the guard enforces `tests-unwritten` restrictions (the most restrictive edit policy), but there's no phase record in `.clc/state` to advance from. The agent can't edit production files and can't transition forward.

The session-start hook resolves this automatically. When it detects a feature branch with no phase, it looks for a [tisket](/tisket/what-is-tisket) issue whose ID matches the branch name. If one exists, it sets the phase to `tests-unwritten` and advances the tisket to `in_progress`, giving the agent a valid starting point. If no matching issue is found, the agent stays in the restricted-but-phaseless state — which effectively means the worktree needs to be created properly via `clc pickup`.

## Custom workflows

Custom workflows are defined in `clc.yml` under the `workflows` key. A
workflow specifies phases, their permissions, their transitions, and
optional review gates.

```yaml
workflows:
  docs-only:
    phases:
      - name: drafting
        instructions: "Write documentation."
        transitions: [review]
      - name: review
        can_stop: true
        transitions:
          - drafting
          - target: done
            requires: [docs]
      - name: done
    reviews:
      docs:
        instructions: "Check for accuracy and completeness."
```

Policy rules select which workflow applies to a given issue:

```yaml
rules:
  - workflow: docs-only
    match:
      label: docs
  - workflow: tdd
    match: {}
```

Rules are evaluated in order. The first match wins. If no rule matches,
clc falls back to a workflow named `"default"` (if defined), then to the
built-in TDD workflow.

## Review gates

Transitions can require reviews before they're allowed. A transition
with `requires: [code]` means the `code` review must pass before the
agent can advance.

Reviews are defined in two places:

- **In the workflow definition** (`workflows.<name>.reviews`) — review
  type name, instructions, and optional permissions.
- **As reviewer files** (`.clc/reviewers/<name>.md`) — a markdown file
  with AgentSpec frontmatter that specifies the model, max turns, and
  review prompt.

When a transition requires a review, the reviewer agent is spawned,
evaluates the work, and returns a verdict (approve or request changes).
If changes are requested, the agent goes back to an earlier phase to
address them.

## Further reading

- [CLI Reference](/clc/cli-reference) — `clc status` and `clc status set` commands
- [Getting Started](/getting-started) — full walkthrough of the pickup-to-done cycle
- [What is codelikecody?](/what-is-codelikecody) — how phases fit into the broader system
