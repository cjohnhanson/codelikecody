---
title: "Prime text should treat todo status as dispatch-ready and enforce scoping"
status: discovery
priority: 2
assignee:
labels: [clc, ergonomics]
depends_on: [restructure-clc-prime-text-as-skills-with-progressive-disclosure]
created: 2026-03-05T04:30:00Z
updated: "2026-03-23T02:14:19Z"
---

## Problem

Moving a tisket to `todo` effectively means "a coordinator can dispatch a worker to implement this right now." But the prime text doesn't communicate this. Agents (and humans) move things to `todo` casually without ensuring the tisket is scoped well enough for autonomous implementation.

## What should happen

The prime/context injection should make clear:

- `discovery` = "needs thinking, not ready for implementation"
- `todo` = "dispatch-ready — a worker should be able to pick this up cold and implement it without asking clarifying questions"
- Moving discovery → todo is a deliberate act that implies the tisket has: a clear scope, acceptance criteria (even if informal), and enough context for someone unfamiliar with the decision history

The agent should double-check before setting status to `todo`:
- Is the scope clear enough for a worker to implement without back-and-forth?
- Are there unresolved design questions that should be settled first?
- Does the body contain enough context, or is it just a title?

This is a context injection / prime text concern — the enforcement is social (prompt-based), not mechanical (hook-based). The phase system gates implementation quality; this gates dispatch quality.

## Depends on

`restructure-clc-prime-text-as-skills-with-progressive-disclosure` — this guidance belongs in the tisket section of the prime tree, loaded when the agent is on trunk doing triage work.
