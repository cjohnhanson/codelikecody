Do not write or modify LLM prompt content without explicit user approval. Propose drafts eagerly, but do not write to file until approved.

Prompt content in this project includes:
- AGENTS.md / CLAUDE.md (agent instructions loaded at session start)
- Prime text (imperative directives injected by clc hooks)
- Hook context injection (SessionStart, reinforcement, Stop messages)
- Documentation that agents consume (docs bundled in the binary)

## CRITICAL: Test failures and compiler warnings

**Test failures and compiler warnings are NEVER acceptable. There are
no exceptions to this rule.**

- Never dismiss a failing test as "pre-existing" or "unrelated to this change."
- Never dismiss a compiler warning as "from another branch" or "not in scope."
- Never say "that's not ours to fix." If it's in the build output, it's yours to fix.
- Never proceed with other work while tests are failing or warnings exist.
- `cargo build --workspace` must produce zero warnings.
- `cargo test --workspace` failures must be investigated and fixed or captured as a tisket.

Every failure. Every warning. Every time. Work is not complete until
the build is clean and tests pass. If something broke from a merge,
fix it — the merge made it yours.

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

## Documentation completeness

When adding or changing user-facing features (new commands, config fields,
assertion types, API changes), the following must be updated before the
work is considered done:

- **Bundled docs** (`missouri/docs/`, `tisket/docs/`, etc.) — CLI reference,
  guides, and conceptual docs shipped with the binary
- **Product skills** (`skills/`) — skills that teach agents how to use the tools
- **Development skills** (`.agents/skills/`) — if the change affects how
  agents develop this project

Documentation is not a follow-up. It ships with the code.
