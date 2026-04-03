---
title: "tisket has zero tests across 2420 lines — parse/serialize round-trip never verified"
status: todo
priority:
assignee:
labels: [tisket, testing, standard]
depends_on: []
created: 2026-03-23T03:12:04Z
updated: "2026-04-03T18:33:27Z"
---

## Problem

1. The `tisket` crate (2,420 lines across 11 source files) implements YAML frontmatter parsing, issue serialization, short-ID generation, repo operations, git integration, status transitions, and a full CLI. Parse/serialize round-trips, ID resolution, status filtering, and repo operations should all have test coverage.
2. There are zero `#[test]` attributes and no `#[cfg(test)]` modules anywhere in `tisket/src/`. The `parse_issue` and `serialize_issue` functions in `issue.rs` (247 lines) handle YAML frontmatter extraction and reconstruction — a round-trip that has never been verified. `repo.rs` (911 lines) implements ID resolution with prefix matching, issue listing with filtering, and scratch note management — all untested. A separate tisket issue (`r2lp`) already documents that `serialize_issue` hand-rolls YAML output bypassing serde.
3. Silent data loss or corruption in tisket's parse/serialize path would affect every issue in every project using clc. Since tisket is the task store that clc, dispatch, and coordinators all depend on, a serialization bug propagates into broken worker prompts, incorrect status transitions, and lost issue bodies.

## Open Questions

- What's the highest-risk surface: parse/serialize round-trip, ID prefix resolution (which has ambiguity handling), or the status transition logic in `edit_issue`?
- Should tests use fixture `.md` files or construct issues programmatically?
- Is the hand-rolled YAML serializer in `serialize_issue` (tisket `r2lp`) a prerequisite fix, or can tests be written against the current behavior and updated later?

## Why It Matters

Tisket is the data layer for the entire clc workflow. Every issue body, status transition, and ID lookup flows through this code. 2,420 lines of data handling with zero tests means any refactor — like the recent YAML config migration — risks silent corruption of the task store.
