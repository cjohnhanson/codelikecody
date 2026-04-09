---
title: "missouri: test database state the same way we test filesystem state (postgres fixtures, diffs)"
status: todo
priority:
assignee:
labels: [missouri, feature]
depends_on: []
created: 2026-04-09T03:42:59Z
updated: "2026-04-09T03:43:38Z"
---

## Problem

Missouri tests filesystem state as a graph. Works great for CLI tools, git repos, flat files. Breaks down for code that reads/writes a database:

- Fresh Postgres per test is slow (seconds of setup)
- Comparing DB state between nodes means dumping and diffing
- Transactions, schemas, sequences, timestamps all complicate reproducibility

## What we want

The same experience as the filesystem approach:
- A graph of states
- Each state is a 'database snapshot' (set of tables with known rows)
- Transitions are SQL commands or application commands
- Diffs between states show what changed
- Assertions can run SQL to verify

## Possible approaches

1. **pg_dump-based snapshots** — each state has a `state.sql` file with `INSERT`s. Before a transition runs, restore the dump. After, pg_dump again and compare. Slow but simple.

2. **Template databases** — Postgres `CREATE DATABASE ... TEMPLATE` for fast forks. Each state is a template. Transitions clone → mutate. Faster but requires Postgres-specific features.

3. **Row-level fixtures** — YAML files per table/state. Missouri loads them into a schema. Diff at the row level, not the SQL level.

4. **Event sourcing** — each state is an append-only log of events. Transitions replay events. State comparison is event comparison.

5. **testcontainers + pg_dump** — spin up a throwaway Postgres container per test path, snapshot via pg_dump between states. Cleaner isolation, slower.

## Integration concerns

- Needs to work on both Linux and macOS (no docker-in-CI weirdness)
- Should reuse as much missouri graph traversal as possible
- Needs to coexist with filesystem state testing (a single test might touch both)
- Deterministic: same input, same output across runs (pairs with jz0h time mocking tisket)

## Acceptance criteria

- Design doc / decision on approach
- Reference implementation testing a simple schema (e.g. coordination_agents table)
- Documentation in missouri guide
