<!-- metadata
title: "The Phase System"
description: "How clc enforces test-driven development through ordered phases"
type: explanation
-->

# The Phase System

The phase system is clc's mechanism for enforcing test-driven development at
the tool level. Rather than relying on an agent's good intentions, phases
constrain which files can be edited and whether the agent is allowed to stop
working. The constraints are enforced by hooks that intercept every tool call
in a session — the agent can't opt out.

## Why phases exist

An unsupervised coding agent, left to its own devices, will skip tests. It
will write implementation first, then either write trivial tests after the
fact or forget them entirely. TDD requires discipline that agents don't
naturally have, so clc imposes it mechanically.

Phases also prevent premature completion. Without the stop guard, an agent
will declare itself done at the first sign of progress — tests passing,
code compiling, whatever feels like a stopping point. The phase system
forces the agent through the full lifecycle before it's allowed to exit.

## The nine phases

Phases are an ordered sequence. Every piece of work moves through them from
beginning to end:

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

## What each phase allows

The guard system splits phases into two categories: unrestricted and
restricted.

**Unrestricted phases** (implementing, in-review): all tool calls pass
through. The agent can edit any file, run any command. These are the
"do the work" phases.

**Restricted phases** (everything else): file-targeting tools — Edit,
Write, NotebookEdit — are blocked unless the target path is inside
`tests/missouri/`. This is how the system forces test-first development:
in the early phases, the only files you can touch are test files. In the
late phases, the implementation is supposed to be done, so production
edits are locked down again.

Bash commands, read-only tools (Read, Glob, Grep), and other non-file-targeting
tools are allowed in all phases. Bash is too hard to gate by file path
reliably, and blocking it entirely would make the agent unable to run tests
or check status. The tradeoff is that a determined agent could technically
write files via Bash in a restricted phase — but the hook system is about
shaping behavior, not building a sandbox.

## The stop guard

The stop hook prevents the agent from ending its session prematurely. On
a feature branch, the agent is only allowed to stop in three phases:

- **review-requested** — work is submitted, waiting on someone else
- **reviewed** — review is done, work is approved
- **done** — everything is finalized

Every other phase blocks the stop event. Notably, **green blocks stop**.
Tests passing is not the finish line. The work still needs to go through
review before the agent is allowed to walk away. This is a deliberate
choice — green feels like "done" to an agent, but it isn't.

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

## Further reading

- [CLI Reference](/clc/cli-reference) — `clc status` and `clc status set` commands
- [Getting Started](/getting-started) — full walkthrough of the pickup-to-done cycle
- [What is codelikecody?](/what-is-codelikecody) — how phases fit into the broader system
