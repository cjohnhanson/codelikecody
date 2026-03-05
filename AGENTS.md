Do not write or modify LLM prompt content without explicit user approval. Propose drafts eagerly, but do not write to file until approved.

Prompt content in this project includes:
- AGENTS.md / CLAUDE.md (agent instructions loaded at session start)
- Prime text (imperative directives injected by clc hooks)
- Hook context injection (SessionStart, reinforcement, Stop messages)
- Documentation that agents consume (docs bundled in the binary)

## Stale binary

After landing worker changes, the system binary on PATH may be stale.
Always `cargo build --workspace` before manual testing or launching
new coordinators/workers. Workers dispatched via `clc dispatch` use
whatever `tisket`/`clc`/`missouri` is on PATH — if that's from a
prior build, new features won't be available.
