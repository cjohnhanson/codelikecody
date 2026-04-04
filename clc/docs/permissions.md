<!-- metadata
title: "Permission Model"
description: "How tool calls are evaluated, granted, denied, and escalated"
type: explanation
-->

# Permission Model

Three layers evaluate whether an agent can perform an action. Each
layer can independently block a tool call, and they operate in
sequence.

## Layer 1: Phase guard

The phase guard runs on every `PreToolUse` event. It loads the
current phase's permissions and evaluates the tool call against them.

The evaluation order:

1. If `CLC_GUARD_OFF=1` is set, everything passes. This is the
   escape hatch for developing clc itself.
2. On trunk (main or admin branch), only read-only tools pass.
   Edit, Write, and NotebookEdit are blocked. Bash is restricted
   to a conservative allowlist.
3. On a feature branch, the current phase's `permissions.deny`
   patterns are checked first. If the tool call matches a deny
   pattern, it's blocked.
4. If deny didn't match, `permissions.allow` patterns are checked.
   If the phase has allow patterns and the tool call doesn't match
   any, it's blocked.
5. If no permissions are defined for the phase, all tools pass.

Pattern format: `"Edit"` matches all Edit calls. `"Edit(tests/**)"` 
matches Edit calls where the file path matches the glob. 
`"Bash(cargo *)"` matches Bash calls where the command starts with
`cargo`.

The guard returns one of three results:
- **Allow** — tool call proceeds, optionally with injected context
- **Block** — tool call rejected, with an explanation message
- **Passthrough** — no opinion, tool call proceeds

## Layer 2: Claude Code settings

Claude Code maintains its own permission system via
`.claude/settings.local.json`. When a worker is dispatched, clc
pre-seeds this file with baseline permissions from the `worker.permissions.default`
config.

If a tool call passes the phase guard but isn't in the Claude Code
settings allow list, Claude Code itself blocks it. The agent sees a
permission denial.

## Layer 3: Permission requests

When a worker is blocked by layer 2 (Claude Code settings), it can
request permission:

```
clc permissions request "Need to run npm install for frontend deps"
```

This creates a permission request and the worker stops. The request
enters the coordinator's decision pipeline:

1. **Auto-grant check** — if the tool pattern matches a
   `coordinator.auto_grant` pattern, it's granted immediately.
2. **Always-escalate check** — if it matches an
   `always_escalate` pattern, it goes straight to the user.
3. **Coordinator judgment** — the coordinator agent evaluates the
   request in context and decides: grant, deny, or escalate.

### Grant

The permission is added to the worker's `settings.local.json` (in
worktree mode) or recorded in the coordination database (in Docker
mode). The worker is resumed and the tool call succeeds on retry.

### Deny

The denial is recorded. The worker is resumed but the permission is
still absent. The worker must find an alternative approach.

### Escalate

The request is written to `.clc/escalations/` (worktree mode) or
the coordination database (Docker mode). The user reviews via:

```
clc permissions inbox
clc permissions grant <worker-id> <permission>
clc permissions deny <worker-id> <reason>
```

## Stop control

The phase guard also evaluates `Stop` events. Each phase declares
`can_stop: true` or `false`. An agent in a phase where stopping is
not allowed receives a block message explaining that work must
continue to the next stoppable phase.

## Configuration summary

| Where | What | Effect |
|-------|------|--------|
| Workflow phase `permissions.deny` | Glob patterns | Blocked by phase guard |
| Workflow phase `permissions.allow` | Glob patterns | Allowed by phase guard |
| `worker.permissions.default` | Tool patterns | Pre-seeded at dispatch |
| `worker.permissions.deny` | Tool patterns | Never granted |
| `coordinator.auto_grant` | Tool patterns | Granted without judgment |
| `coordinator.always_escalate` | Tool patterns | Escalated to user |
