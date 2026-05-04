---
title: "evaluate dropping moose fork in favor of upstream agent-browser"
status: in_progress
priority:
assignee:
labels: [decision, moose]
depends_on: []
created: 2026-04-04T12:51:02Z
updated: "2026-05-04T00:36:12Z"
---

## Problem

Moose is a fork of vercel-labs/agent-browser. Maintaining a fork
carries ongoing cost: tracking upstream changes, diverging APIs,
duplicating bug fixes. Forking when changes could be contributed
upstream is poor open source practice.

The additions since forking (Lightpanda support, Appium/WebDriver,
test infrastructure via missouri) may be upstreamable. If they are,
the fork is unnecessary overhead. If they aren't, the fork needs
explicit justification.

## Open Questions

- What exactly changed since fork? `git diff` against upstream HEAD.
- Which changes are upstreamable as PRs to vercel-labs/agent-browser?
- Is there anything moose-specific that can't live upstream?
  (e.g., missouri test integration, daemon socket path conventions)
- What's the maintenance cost of carrying the fork? How often does
  upstream release, and how painful are merges?
- Does vercel-labs/agent-browser accept external contributions?

## Why It Matters

Every hour spent maintaining fork-specific code is an hour not spent
on the core tools. If upstream accepts the changes, the maintenance
burden drops to zero and the broader ecosystem benefits.

If the answer is "drop the fork," moose goes away and the project
depends on agent-browser directly. The moose name and README would
be replaced with integration documentation.

## Scratch Notes
