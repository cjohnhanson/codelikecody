---
title: "Move ClaudeCodeAgent into claude-code crate, unify with Session command builder"
status: todo
priority:
assignee:
labels: [agent, claude-code, clc-sdk]
depends_on: []
created: "2026-04-03T14:52:50Z"
updated: "2026-04-03T14:52:50Z"
---

## Summary

ClaudeCodeAgent lives in clc-sdk but hand-rolls Claude Code CLI command construction, duplicating knowledge that already exists in claude-code::Session::build_command(). Two independent command builders for the same binary.

## Problem

Claude Code-specific knowledge (--allowedTools, --max-budget-usd, --dangerously-skip-permissions, --model, stream-json flags) is scattered across both crates:
- claude-code::SessionConfig has some flags
- clc-sdk::ClaudeCodeAgent::build_start_command has the same flags built independently
- clc-sdk::AgentConfig accumulates more Claude Code-specific fields over time

## Proposed fix

Move ClaudeCodeAgent into the claude-code crate. The Agent trait stays in clc-sdk as the abstraction boundary, but the concrete Claude Code implementation lives where the CLI knowledge lives. ClaudeCodeAgent should use Session::build_command (or a shared helper) instead of duplicating command construction.

This keeps the dependency direction clean: claude-code knows how to talk to Claude Code, clc-sdk defines the trait, consumers depend on both.
