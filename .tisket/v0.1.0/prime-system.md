---
title: "Prime system"
status: discovery
priority:
assignee:
labels: [architecture]
depends_on: [clc-sdk-crate-with-agent-detection]
created: "2026-02-24T14:52:06Z"
updated: "2026-02-24T14:52:06Z"
---

Implement the `prime` subcommand and `ClcTool::prime()` trait method across the
ecosystem. Prime text is imperative — it tells the agent what it must do, not
what it can do.

## Prime hierarchy

Each level of the subcommand tree can have its own prime:

- `clc prime` — workflow enforcement (phases, worktrees, lifecycle)
- `clc tisket prime` — issue tracking requirements
- `clc tisket scratch prime` — scratch/working memory requirements
- `clc missouri prime` — testing requirements

## How prime is used

clc orchestrates which primes to inject and when, mapped to the agent lifecycle:

- **SessionStart**: inject relevant primes based on workflow state
- **Periodic reinforcement**: inject status_basic, possibly re-prime

The `prime` subcommand also works standalone for manual inspection.

## Implementation

1. Each tool implements `ClcTool::prime()` returning its imperative text
2. clc adds a `prime` subcommand that aggregates primes from mounted tools
3. Hook handlers call `prime()` at appropriate lifecycle points
4. Agent detection controls output format (plain markdown vs rendered)

## Note

Actual prime text content requires explicit user approval before being written
to any file. Draft eagerly, write only after approval.
