---
title: "Coordinator should auto-dispatch todo tiskets without asking for permission"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-02T04:09:33Z"
updated: "2026-03-02T04:09:33Z"
---

The coordinator's job is to manage workers autonomously. Currently it finds todo tiskets and then asks "Want me to dispatch workers for these?" — defeating the purpose of having a coordinator at all.

## Expected behavior

When the coordinator sees todo tiskets, it should dispatch workers immediately. No asking, no waiting for confirmation. The whole point of the coordinator is autonomous operation.

## When to pause

The only reason to pause before dispatching is if the tisket explicitly flags something that needs human judgment — e.g., prompt content that requires user approval per CLAUDE.md. This should be the exception, not the default.

## Fix

This is a coordinator prompt change. The system prompt needs to be more directive: "When you see todo tiskets, dispatch workers for them. Do not ask for permission to dispatch — that is your primary function."

Requires user approval before writing (CLAUDE.md rule on prompt content).
