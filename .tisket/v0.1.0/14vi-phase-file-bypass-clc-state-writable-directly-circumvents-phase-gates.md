---
title: "Phase file bypass: .clc/state writable directly, circumvents phase gates"
status: discovery
priority:
assignee:
labels: [bug]
depends_on: []
created: 2026-03-20T03:07:05Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

Phase gates should enforce a strict sequential workflow — an agent cannot skip from `tests-unwritten` to `implementing` without passing through intermediate phases, and forward transitions are validated by `phase::set()` which checks ordinals and attempt counts.

In practice, the phase state lives in a plain text file at `.clc/state` (`phase::write_state` writes it, `phase::load_state` reads it). Any agent with filesystem write access can write `phase: green` directly to `.clc/state`, bypassing `phase::set()` validation entirely. The guard system (`guard.rs`) checks the phase returned by `phase::load()` but has no way to verify the phase was reached through legitimate transitions.

An agent that bypasses phase gates can skip TDD enforcement, reach `done` without writing tests, or jump to `implementing` immediately — defeating the entire purpose of the phase system.

## Open Questions

- Should `.clc/state` be protected via `permissions.deny` rules in the worker's settings.local.json, or does it need a different mechanism (e.g., signed state, integrity check)?
- Would a transition log (append-only) provide auditability even if the state file is tampered with?
- Does the `CLC_GUARD_OFF` escape hatch (guard.rs line 53) compound this risk by also disabling all guard checks via a single env var?

## Why It Matters

Phase gates are the primary mechanism preventing agents from skipping TDD and review steps. If an agent can trivially bypass them by writing a file, the workflow enforcement is security theater — it constrains well-behaved agents but provides no protection against misbehaving ones.
