# clc-sdk

Traits for workspace lifecycle, agent integration, and coordination.

The `Workspace` trait abstracts over isolation backends. Git worktrees
and Docker containers implement it today; the trait is the extension
point for other backends. `ClcTool` defines how tools report status and
phase-aware directives to the workflow engine. `Agent` abstracts over
coding agent processes (currently Claude Code, designed for others).

Also provides coordination primitives (inbox/outbox message queues,
agent specs) and re-exports protocol types from the `claude-code` crate.
