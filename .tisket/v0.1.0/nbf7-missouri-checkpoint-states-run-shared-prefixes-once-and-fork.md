---
title: "missouri subgraph execution — break graph at complete fixtures"
status: todo
priority: 2
assignee:
labels: [missouri]
depends_on: []
created: "2026-03-20T03:37:45Z"
updated: "2026-03-20T03:37:45Z"
---

## Problem

Missouri currently enumerates all paths through the graph and runs each
one end-to-end. In the clc test suite, 31 of 33 paths share the prefix
`bare-project → initialized`. That prefix runs 31 times — once per path —
even though all 31 runs produce the same result. Full suite takes ~20
minutes; ~13 of that is redundant `bare-project → initialized` executions.

## Design: subgraph execution

State fixtures are reference states — they represent the expected
filesystem after a transition. If a fixture is **complete** (contains
everything a downstream transition needs to run, not just what the
comparator checks), it can serve as a starting point for paths.

Missouri should break the graph into subgraphs at states whose fixtures
are complete starting points. Within each subgraph, full path traversal
happens (sequential chaining, same as today). But paths don't traverse
the entire graph from the root — they start from the nearest complete
fixture.

### Example: clc test suite

Today: 31 paths, each running `bare-project → initialized → ...`,
re-executing the prefix every time.

With subgraphs:
- `bare-project → initialized` is one subgraph (1 path, runs once,
  verifies `clc init` produces the expected state)
- 31 paths start directly from the `initialized` fixture — copy it,
  run `initialized → has-config` or `initialized → ready-to-pickup →
  dispatched → ...` as a sequential chain

The fixture IS the trusted starting state. The upstream subgraph's test
already proves the transition produces it correctly. Downstream subgraphs
don't re-derive it.

### What makes a fixture "complete"

A complete fixture contains everything downstream transitions need to
execute — not just the files the comparator checks. For example, the
`initialized` fixture currently omits `.git/` (it's ignored in
comparators) but downstream transitions need a git repo. Making the
fixture complete means adding `.git/` to it.

States with ignored paths in their incoming transition comparators are
the ones most likely to have incomplete fixtures. Fixing those fixtures
is part of this work.

### Implementation

1. Make the `initialized` fixture complete (add `.git/`, any other
   missing runtime state)
2. Decide how missouri identifies subgraph roots — explicit annotation
   in `missouri.yml`, implicit detection from graph structure, or both
3. Modify path enumeration to enumerate within subgraphs rather than
   across the full graph
4. Each subgraph's paths run in parallel (same as today's `par_iter`)
5. Reporting stays path-based — reassemble subgraph results into full
   path displays

### Expected impact

clc suite: ~20 min → ~2-3 min (31 × 25s prefix eliminated, replaced by
31 parallel paths of 1-5s each starting from fixture)
