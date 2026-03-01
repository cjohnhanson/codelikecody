# clc

Workflow engine for coding agents. Phase-gated TDD, worktree isolation, hook system, worker orchestration.

Agents work in isolated git worktrees, constrained by a phase system that enforces test-first development. Hooks inject context and block disallowed actions at each phase. Workers run Claude Code processes and can be dispatched, monitored, stopped, and resumed.
