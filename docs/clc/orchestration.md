<!-- metadata
title: "Multi-Agent Orchestration"
description: "How to dispatch, monitor, and land multiple coding agents"
type: guide
-->

# Multi-Agent Orchestration

## Dispatch a Single Worker

Run from the main branch. The tisket must exist and have `todo` status with
all dependencies resolved.

```
clc dispatch <tisket-id>
```

This creates a worktree at `.worktrees/<tisket-id>/`, picks up the tisket,
seeds baseline permissions, and spawns a background Claude process.

To use a specific model (default is `sonnet`):

```
clc dispatch <tisket-id> --model opus
```

If a stale worktree exists from a prior failed run, dispatch cleans it up
automatically and re-dispatches.

## Launch a Coordinator

A coordinator is an agent that dispatches and monitors workers on your behalf.
It runs on the main branch.

```
clc coordinate
```

The coordinator finds all pickable tiskets (status `todo`, dependencies
resolved), dispatches workers for them, monitors progress, and lands
completed work.

### Filter Which Tiskets Get Picked Up

By label:

```
clc coordinate --label backend
clc coordinate --exclude-label blocked
```

By project:

```
clc coordinate --project v0.1.0
```

By dependency chain (only tiskets that transitively depend on a root tisket):

```
clc coordinate --depends-on root-tisket-id
```

By a single specific tisket:

```
clc coordinate --tisket specific-tisket-id
```

By comma-separated selectors (AND composition):

```
clc coordinate --filter "label:feature,project:v0.1.0"
```

### Dry Run

Preview which tiskets would be dispatched without launching anything:

```
clc coordinate --dry-run
```

Combine with filters to verify selector behavior:

```
clc coordinate --label backend --dry-run
```

### Named Coordinators

Give a coordinator an identity so workers know who dispatched them:

```
clc coordinate --id coord-backend --label backend
```

### Model Selection

Workers spawned by the coordinator default to `opus`. Override with:

```
clc coordinate --model sonnet
```

## Monitor Workers

### List All Workers

```
clc workers
```

Shows live workers. To include dead ones:

```
clc workers --all
```

Output is tab-separated: ID, status, PID, line count, last activity type.

### Check Recent Activity

Cursor-based — shows only output since the last check:

```
clc worker <id> check
```

If the worker has a pending permission request, `check` surfaces it.

### View Output Log

Last 50 lines (parsed):

```
clc worker <id> log
```

More or fewer lines:

```
clc worker <id> log --lines 200
```

### Raw NDJSON Output

```
clc worker <id> raw
clc worker <id> raw --lines 0   # all output
```

### Send a Message to a Worker

```
clc worker <id> send "focus on the edge case in parse_config"
```

The worker must be alive. Messages arrive on its stdin pipe.

## Monitor Coordinators

### List Coordinators

```
clc coordinators
clc coordinators --all   # include dead
```

### Check, Log, Send

Same interface as workers, scoped to a coordinator ID:

```
clc coordinator <id> check
clc coordinator <id> log --lines 100
clc coordinator <id> send "pause dispatching until tests pass"
```

## Handle Permissions

Workers start with a baseline set of permissions (file operations, search,
clc/tisket/missouri/cargo commands, basic shell). When a worker needs
something beyond that, it files a request and stops.

### See Pending Requests

```
clc permissions list
```

### Grant a Permission

```
clc permissions grant <worker-id> "Bash(npm install:*)"
```

This adds the rule to the worker's `.claude/settings.local.json` and removes
the pending request. Then resume the worker:

```
clc worker <worker-id> resume
```

### Escalate to the User

The coordinator can escalate a decision it doesn't want to make:

```
clc permissions escalate <worker-id> "wants to run docker build — unclear if safe"
```

### Review Escalations

```
clc permissions inbox
```

### Deny a Request

```
clc permissions deny <worker-id> "docker is not available in this environment"
```

Then resume the worker so it can adjust its approach:

```
clc worker <worker-id> resume
```

### Configure Permission Policy

In `clc.yml`, define what the coordinator auto-grants vs. always escalates:

```yaml
coordinator:
  auto_grant:
    - "Bash(cargo *)"
    - "Bash(npm *)"
  always_escalate:
    - "Bash(rm *)"
    - "Bash(git push *)"
```

Override or extend via CLI flags:

```
clc coordinate --auto-grant "Bash(make *)"
clc coordinate --escalate-all   # escalate everything
```

Or provide an external policy file:

```
clc coordinate --grant-config policy.yml
```

Where `policy.yml` has the same shape:

```yaml
auto_grant:
  - "Bash(docker *)"
always_escalate:
  - "Bash(sudo *)"
```

Merging order: config file, then `--grant-config`, then `--auto-grant` flags.

### Configure Worker Permissions

In `clc.yml`, override the default permission set seeded into every worker:

```yaml
worker:
  permissions:
    default:
      - Read
      - Grep
      - Glob
      - "Write({worktree}/**)"
      - "Edit({worktree}/**)"
      - "Bash(clc *)"
      - "Bash(cargo *)"
    deny:
      - "Write({worktree}/.clc/**)"
      - "Edit({worktree}/.clc/**)"
```

`{worktree}` expands to the actual worktree path at dispatch time.

When `default` is empty, the hardcoded baseline is used (Read, Write, Edit,
Grep, Glob, WebFetch, WebSearch, clc/tisket/missouri/cargo commands, basic
git and filesystem operations).

## Land Completed Workers

A worker is ready to land when it reaches the `done` phase. Landing stops
the worker (if alive), merges its branch into trunk, and cleans up the
worktree.

```
clc land <id>
```

If `clc land` fails with "not a descendant of HEAD," that means trunk advanced
since the worker branched. `clc land` handles this by rebasing automatically.

## Stop and Resume Workers

### Stop

Sends SIGTERM, waits up to 2 seconds, then SIGKILL if needed. Worktree stays
intact.

```
clc worker <id> stop
```

### Resume

Re-attaches to the worker's existing Claude session:

```
clc worker <id> resume
```

### Supervise

Auto-resumes a worker if it stops before reaching `done`:

```
clc worker <id> supervise
clc worker <id> supervise --max-resumes 5
```

Supervision blocks on pending permission requests — it won't resume a
worker that's waiting for a grant.

## Recover Stranded Workers

A stranded worker is one where the process died but work was partially
completed (phase is set but not `done`).

### Find Stranded Workers

```
clc workers --stranded
```

### Recover a Worker at Green Phase

If a worker reached `green` (tests passing) but died before running
`clc done`, recovery advances it through the remaining phases:

```
clc worker <id> recover
```

This only works at the `green` phase. Workers stranded at earlier phases
need to be resumed instead:

```
clc worker <id> resume
```

### Clean Up Dead Worker State

```
clc workers --prune
```

Removes `.clc/worker/` state files from worktrees where the process is dead.

## Declarative Orchestration with clc.yaml

The topology file `clc.yaml` at the project root declares the full
multi-agent setup: workspaces, coordinators, inboxes, outboxes, and an
optional admin agent.

```yaml
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
  reviewer:
    type: reviewer
    agent: claude-opus-4-6

coordinators:
  backend:
    workspace: worker
    selector:
      label: backend
      exclude_label: blocked
  frontend:
    workspace: worker
    selector:
      label: frontend

inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user

outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker

admin:
  prompt: You are the admin agent.
  inboxes: [user-inbox]
  outboxes: [worker-outbox]
  coordinators: [backend, frontend]
```

### Topology Structure

**workspaces** define agent configurations. Each has a `type` (`worker` or
`reviewer`) and an `agent` (model name).

**coordinators** reference a workspace and optionally include a `selector`
to filter tiskets. Selector fields: `label`, `exclude_label`, `project`,
`depends_on`.

**inboxes** and **outboxes** define message channels. Currently supports
`folder_watch` (reads from a directory) and `folder_write` (writes to a
directory).

**admin** ties it all together: a prompt, which inboxes and outboxes it
uses, and which coordinators it manages.

### Validation

Cross-references are validated on load. A coordinator that references a
nonexistent workspace, or an admin that references a nonexistent inbox,
outbox, or coordinator, produces an error.

## Integration Branches

For coordinated multi-worker merges, use integration branches:

```
clc integrate create release-batch
clc integrate merge <worker-branch>
clc integrate merge <another-worker-branch>
clc integrate land
```

`land` squash-merges the integration branch onto main.

Coordinators can also be landed, which squash-merges their integration
branch:

```
clc coordinator <id> land
```
