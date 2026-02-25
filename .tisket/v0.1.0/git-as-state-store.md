---
title: "Git as state store"
status: discovery
priority:
assignee:
labels: [architecture]
depends_on: []
created: "2026-02-25T00:00:00Z"
updated: "2026-02-25T00:00:00Z"
---

Core architectural principle: in clc-managed projects, git is the primary store
of state. Commits are not a byproduct of working — they are the point. Each
commit is a state snapshot and a validation checkpoint.

## Principles

- Commits should be frequent. Low threshold. Every logical pause is a commit.
- Projects using clc should have good pre-commit hooks. Frequent commits mean
  hooks run often, acting as a continuous quality ratchet.
- The commit history becomes a legible record of how work progressed — not just
  what changed, but the sequence of validated states it passed through.
- Phase transitions, test results going green, logical units completing — all
  natural commit points.

## Relationship to clc

- clc already stores phase in `.clc/state` — but the richer state is the git
  tree itself (what files exist, what they contain, what tests pass).
- The deterministic-test-result-caching tisket explores tying missouri results
  to git tree state — this is the same principle applied to test caching.
- `clc commit` could be the opinionated entry point: stage, commit, let hooks
  validate, checkpoint.
- Hook nudging (PostToolUse, UserPromptSubmit) should encourage committing at
  natural pause points rather than after arbitrary edit counts.

## Relationship to pre-commit hooks

- clc init could scaffold or recommend pre-commit hook setup
- The hooks themselves are project-specific (fmt, lint, test, clippy, etc.)
  but clc benefits from their existence
- More commits × good hooks = continuous validation without explicit test runs
