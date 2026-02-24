# clc

## Context Injection Matrix

What gets injected into the agent's context window, by hook event and workflow state.

| | **Main** | **Feature (no phase)** | **tests-unwritten** | **tests-written** | **red** | **implementing** | **green** | **done** |
|---|---|---|---|---|---|---|---|---|
| **SessionStart** | worktree guidance | branch name only | ? | ? | ? | ? | ? | ? |
| **UserPromptSubmit** | — | — | — | — | — | — | — | — |
| **PreToolUse** | BLOCK writes | passthrough | BLOCK non-test edits | BLOCK non-test edits | BLOCK non-test edits | passthrough | BLOCK non-test edits | BLOCK non-test edits |
| **PostToolUse** | — | — | — | — | — | — | — | — |
| **PostToolUseFailure** | — | — | — | — | — | — | — | — |
| **Stop** | — | — | — | — | — | — | — | — |

**Legend:**
- `BLOCK` = action is blocked, agent receives error message with guidance
- `—` = passthrough, no context injected
- `?` = not yet implemented
