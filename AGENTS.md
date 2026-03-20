Do not write or modify LLM prompt content without explicit user approval. Propose drafts eagerly, but do not write to file until approved.

Prompt content in this project includes:
- AGENTS.md / CLAUDE.md (agent instructions loaded at session start)
- Prime text (imperative directives injected by clc hooks)
- Hook context injection (SessionStart, reinforcement, Stop messages)
- Documentation that agents consume (docs bundled in the binary)

## Test failures

A test failure is a test failure. Never dismiss a failing test as
"pre-existing" or "unrelated to this change." Every failure must be
investigated and either fixed or captured as a tisket. Work is not
complete while tests are failing.

## Stale binary

After landing worker changes, the system binary on PATH may be stale.
Always `cargo build --workspace` before manual testing or launching
new coordinators/workers. Workers dispatched via `clc dispatch` use
whatever `tisket`/`clc`/`missouri` is on PATH — if that's from a
prior build, new features won't be available.

## Skills

Two skill directories serve different audiences:

- **`skills/`** — Skills for agents *using* this project's tools (missouri,
  tisket, clc). Product documentation for consumers. These ship with the
  project and teach agents how to use what's built here.

- **`.agents/skills/`** — Skills for agents *developing* this project.
  Internal conventions, development practices, repo-specific patterns.
  Symlinked to `.claude/skills/` for Claude Code integration.

### Continuous improvement

When a skill causes friction — wrong instructions, missed cases, the user
corrects something a skill recommended — update the skill immediately.
Skills rot. Commands get renamed, patterns evolve, tools change defaults.
A skill that was correct last month may not be correct now. When touching
a skill for any reason, verify its claims still hold. If the fix is
non-trivial, create a tisket to track it.
