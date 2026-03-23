<!-- metadata
title: "clc CLI Reference"
description: "Complete command reference for the clc workflow engine"
type: reference
-->

# clc CLI Reference

```
clc <command>
```

Workflow enforcement for coding agents. Every subcommand operates within a git repository that has been initialized with `clc init`.

---

## clc init

Initialize clc in the current project directory. Sets up `.clc/` state directory and configures agent hooks.

```
clc init [--untracked] [--force]
```

| Flag | Description |
|------|-------------|
| `--untracked` | Keep clc files invisible to git via `.git/info/exclude` |
| `--force` | Overwrite existing hooks in `settings.local.json` |

## clc hook

Process a hook event from stdin. Called by agent hooks (e.g., Claude Code's `PreToolUse`, `PostToolUse`, `Stop`), not invoked directly.

```
clc hook
```

Reads the hook payload from stdin and emits directives to stdout.

## clc status

Show current clc state: branch, workflow phase, and related metadata.

```
clc status
clc status set <phase>
```

### clc status set

Set the current workflow phase manually.

| Argument | Description |
|----------|-------------|
| `<phase>` | The phase to transition to |

## clc pickup

Pick up a tisket: creates a worktree, sets status, and initializes the workflow phase.

```
clc pickup <id>
```

| Argument | Description |
|----------|-------------|
| `<id>` | The tisket issue ID to pick up |

## clc admin

Create or switch to the admin worktree for non-feature work.

```
clc admin
```

## clc home

Print the main repository root path. Useful for navigating back to trunk from a worktree.

```
clc home
```

## clc merge

Merge a completed feature branch into trunk.

```
clc merge <id>
```

| Argument | Description |
|----------|-------------|
| `<id>` | The branch (tisket ID) to merge |

## clc done

Finalize work: advance phase to done and close the tisket.

```
clc done
```

## clc prime

Print the assembled prime text (agent orientation + directives) to stdout. Used to inspect what an agent sees at session start.

```
clc prime
```

## clc config

View and manage clc configuration.

clc has two separate config concepts:

**Topology** (`clc.yaml`) declares workspace members for multi-crate coordination. Read by `clc coordinate` to discover member suites. This file has nothing to do with project settings.

**Project config** controls settings like main branch, worker permissions, and skill sources. Three files are checked in priority order:

1. `clc.yml` — at the repo root. Highest priority.
2. `clc.toml` — at the repo root. Used if `clc.yml` doesn't exist.
3. `.clc/config.yml` — inside the `.clc/` state directory. Legacy location, lowest priority.

### clc config show

Print the effective configuration.

```
clc config show
```

---

## Coordination

### clc coordinate

Run the coordinator: dispatch pickable tiskets to worker agents.

```
clc coordinate [options]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--model <model>` | String | `opus` | Model to use for workers |
| `--tisket <id>` | String | | Only process this specific tisket |
| `--label <label>` | String | | Only tiskets with this label |
| `--exclude-label <label>` | String | | Skip tiskets with this label |
| `--project <name>` | String | | Only tiskets in this project |
| `--depends-on <id>` | String | | Only tiskets in the dependency chain rooted at this ID |
| `--filter <selectors>` | String | | Filter by comma-separated selectors (e.g. `label:feature,project:v0.1.0`) |
| `--dry-run` | bool | `false` | List pickable tiskets and exit without spawning |
| `--id <id>` | String | | Unique identity for this coordinator (e.g., `coord-infra`) |
| `--auto-grant <pattern>` | String | | Permission pattern to auto-grant to workers (repeatable) |
| `--escalate-all` | bool | `false` | Escalate all permission requests to the user |
| `--grant-config <path>` | String | | Path to an external permission policy YAML file |

### clc dispatch

Dispatch a worker: pickup a tisket and spawn a detached claude process.

```
clc dispatch <id> [--model <model>] [--coordinator-id <id>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `<id>` | String | *required* | The tisket issue ID to dispatch |
| `--model <model>` | String | `sonnet` | Model to use for the worker |
| `--coordinator-id <id>` | String | | Coordinator ID claiming this tisket |

### clc coordinators

List running coordinators with status.

```
clc coordinators [--all]
```

| Flag | Description |
|------|-------------|
| `--all` | Show all coordinators including dead ones |

### clc coordinator

Interact with a specific coordinator.

```
clc coordinator <id> <action>
```

| Argument | Description |
|----------|-------------|
| `<id>` | The coordinator ID |

#### clc coordinator \<id\> check

Show activity since last check (cursor-based).

#### clc coordinator \<id\> log

Show parsed output log.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--lines <n>` | usize | `50` | Number of lines to show |

#### clc coordinator \<id\> send

Send a follow-up message to the coordinator.

| Argument | Description |
|----------|-------------|
| `<message>` | The message to send |

#### clc coordinator \<id\> stop

Stop the coordinator process.

#### clc coordinator \<id\> land

Squash-merge the coordinator's integration branch into main.

---

## Workers

### clc workers

List active workers and their status.

```
clc workers [--all] [--prune] [--stranded]
```

| Flag | Description |
|------|-------------|
| `--all` | Show all workers including dead ones |
| `--prune` | Remove worker state files for dead workers |
| `--stranded` | Show stranded workers: worktrees with no alive process and a phase set |

### clc worker

Interact with a specific worker.

```
clc worker <id> <action>
```

| Argument | Description |
|----------|-------------|
| `<id>` | The worker ID (tisket ID) |

#### clc worker \<id\> check

Show activity since last check (cursor-based).

#### clc worker \<id\> log

Show parsed output log.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--lines <n>` | usize | `50` | Number of lines to show |

#### clc worker \<id\> send

Send a follow-up message to the worker.

| Argument | Description |
|----------|-------------|
| `<message>` | The message to send |

#### clc worker \<id\> stop

Stop the worker process. Leaves the worktree intact.

#### clc worker \<id\> resume

Resume a stopped worker by re-attaching to the existing session.

#### clc worker \<id\> supervise

Auto-resume a worker if it stops before reaching the done phase.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--max-resumes <n>` | u32 | `3` | Maximum number of auto-resumes before giving up |

#### clc worker \<id\> raw

Show raw NDJSON output.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--lines <n>` | usize | `10` | Number of lines to show from end. `0` = all |

#### clc worker \<id\> recover

Recover a stranded worker: finalize work without re-dispatching.

### clc land

Land a completed worker: stop, verify, merge, cleanup.

```
clc land <id>
```

| Argument | Description |
|----------|-------------|
| `<id>` | The worker ID (tisket ID) to land |

---

## Permissions

### clc permissions request

Request a permission from the coordinator. Called from within a worker worktree.

```
clc permissions request <description>
```

| Argument | Description |
|----------|-------------|
| `<description>` | What permission is needed and why |

### clc permissions grant

Grant a permission to a worker. Called by the coordinator.

```
clc permissions grant <worker_id> <permission>
```

| Argument | Description |
|----------|-------------|
| `<worker_id>` | The worker ID (tisket ID) |
| `<permission>` | The permission to grant (e.g., `Bash(npm install:*)`) |

### clc permissions list

List all pending permission requests across workers.

```
clc permissions list
```

### clc permissions escalate

Escalate a permission decision to the user. Called by the coordinator.

```
clc permissions escalate <worker_id> <description>
```

| Argument | Description |
|----------|-------------|
| `<worker_id>` | The worker ID (tisket ID) |
| `<description>` | What the worker needs and why it requires user review |

### clc permissions inbox

View pending escalations from the coordinator.

```
clc permissions inbox
```

### clc permissions deny

Deny a permission escalation.

```
clc permissions deny <worker_id> <reason>
```

| Argument | Description |
|----------|-------------|
| `<worker_id>` | The worker ID (tisket ID) |
| `<reason>` | Reason for denying the permission request |

---

## Messaging

### clc inbox poll

Poll the inbox directory (`.clc/inbox/`), printing items as JSON and moving them to `.processed/`.

```
clc inbox poll
```

### clc outbox write

Write an item to the outbox (`.clc/outbox/`), reading content from stdin.

```
clc outbox write <name>
```

| Argument | Description |
|----------|-------------|
| `<name>` | Filename for the item (e.g., `summary.md`, `result.json`) |

---

## Integration Branches

Ephemeral integration branches for combining multiple worker branches before landing on main.

### clc integrate create

Create a new integration branch at the current main HEAD.

```
clc integrate create <name>
```

Creates `integrate/<name>`.

| Argument | Description |
|----------|-------------|
| `<name>` | Name for the integration branch |

### clc integrate merge

Merge a worker branch into the current integration branch.

```
clc integrate merge <branch>
```

| Argument | Description |
|----------|-------------|
| `<branch>` | The branch name to merge |

### clc integrate land

Squash-merge the integration branch onto main and clean up.

```
clc integrate land
```

---

## clc tisket

Delegated subcommands for the tisket plaintext issue tracker. When invoked through `clc tisket`, the `--root` flag defaults to the current directory.

### clc tisket init

Initialize tisket in the current repository.

```
clc tisket init
```

### clc tisket prime

Print agent instructions to stdout.

```
clc tisket prime
```

### clc tisket hooks setup

Set up hooks for a coding agent.

```
clc tisket hooks setup <agent> [--scope <scope>]
```

| Argument/Flag | Type | Default | Description |
|---------------|------|---------|-------------|
| `<agent>` | String | *required* | Agent to configure (e.g., `claude`) |
| `-s, --scope <scope>` | `local\|project\|user` | `local` | Configuration scope |

Scopes: `local` = `.claude/settings.local.json` (gitignored), `project` = `.claude/settings.json` (version controlled), `user` = `~/.claude/settings.json` (global).

### clc tisket issue create

```
clc tisket issue create <title> [options]
```

| Flag | Type | Description |
|------|------|-------------|
| `-p, --project <name>` | String | Project to create in (default: root `.tisket/`) |
| `--priority <n>` | u8 | Priority: 1=urgent, 2=high, 3=medium, 4=low |
| `-a, --assignee <name>` | String | Assignee |
| `-l, --labels <csv>` | String | Comma-separated labels |
| `-d, --depends-on <csv>` | String | Comma-separated issue IDs this depends on |
| `--due <date>` | String | Due date (YYYY-MM-DD) |
| `-s, --status <status>` | String | Initial status (default: `todo`) |
| `-b, --body <text>` | String | Issue body text (inline) |
| `--body-file <path>` | Path | Read issue body from file (mutually exclusive with `--body`) |

### clc tisket issue list

```
clc tisket issue list [options]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-p, --project <name>` | String | all | Filter to a project |
| `-s, --status <status>` | String | | Filter by status |
| `-a, --assignee <name>` | String | | Filter by assignee |
| `--label <label>` | String | | Filter by label |
| `--closed` | bool | `false` | Include closed issues |
| `--format <fmt>` | `text\|json` | `text` | Output format |

### clc tisket issue show

```
clc tisket issue show <id> [--format <fmt>] [--field <name>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--format <fmt>` | `text\|json` | `text` | Output format |
| `--field <name>` | String | | Extract a single field value |

Valid `--field` values: `title`, `status`, `priority`, `assignee`, `due_date`, `labels`, `depends_on`, `body`, `scratch`, `id`, `project`.

### clc tisket issue path

Print the file path of an issue.

```
clc tisket issue path <id>
```

### clc tisket issue edit

```
clc tisket issue edit <id> [options]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title <text>` | String | New title |
| `-s, --status <status>` | String | New status |
| `--priority <n>` | u8 | New priority |
| `-a, --assignee <name>` | String | New assignee |
| `-l, --labels <csv>` | String | New labels (replaces existing) |
| `--add-label <label>` | String | Add a label (keeps existing) |
| `--remove-label <label>` | String | Remove a label (keeps others) |
| `-d, --depends-on <csv>` | String | New dependencies (replaces existing) |
| `--due <date>` | String | Due date (YYYY-MM-DD) |
| `--body <text>` | String | Replace the entire body below frontmatter |
| `--append <text>` | String | Append text to the body |

### clc tisket issue close

```
clc tisket issue close <id> [-p <project>] [-s <status>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-p, --project <name>` | String | | Project containing the issue |
| `-s, --status <status>` | String | `done` | Terminal status |

### clc tisket issue reopen

```
clc tisket issue reopen <id> [-s <status>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-s, --status <status>` | String | `todo` | Status to reopen as |

### clc tisket issue move

Move an issue to a different project.

```
clc tisket issue move <id> --project <name>
```

### clc tisket search

Search issues by regex pattern.

```
clc tisket search <pattern> [-p <project>]
```

| Flag | Type | Description |
|------|------|-------------|
| `-p, --project <name>` | String | Filter to a specific project |

### clc tisket scratch

Read or modify scratch notes for an issue. With no subcommand, prints the scratch notes (same as `read`).

```
clc tisket scratch <id> [read|append|write|clear]
```

- `read` -- Print scratch notes (default)
- `append <text>` -- Append text to scratch notes
- `write <text>` -- Replace scratch notes with text
- `clear` -- Clear scratch notes

### clc tisket project create

```
clc tisket project create <name>
```

### clc tisket project list

```
clc tisket project list
```

---

## clc missouri

Delegated subcommands for missouri, the e2e testing framework based on directed graphs of filesystem states.

Global flags available on all missouri subcommands:

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-C <dir>` | Path | | Change to this directory before doing anything |
| `--config-dir <name>` | String | `.missouri` | Name of the config directory |

### clc missouri run

Run all test paths.

```
clc missouri run [options]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-d, --dir <path>` | Path | `.` | Root directory containing states |
| `-v, --verbose` | count | 0 | Increase verbosity (`-v`, `-vv`, `-vvv`) |
| `-q, --quiet` | bool | `false` | Suppress non-essential output |
| `--keep-temp` | bool | `false` | Keep temp directories after run (for debugging) |
| `--check-only` | bool | `false` | Run only state assertions (skip transitions and filesystem comparison) |
| `--no-check` | bool | `false` | Skip all assertions (run only transitions and filesystem comparison) |
| `--record` | bool | `false` | Record transition output to asciicast files |
| `--run-id <id>` | String | timestamp | Custom run ID for recording output directory (requires `--record`) |

`--check-only` and `--no-check` are mutually exclusive. `--record` conflicts with `--check-only`.

### clc missouri list

List states, transitions, or test paths.

```
clc missouri list [-d <path>] [--show <kind>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-d, --dir <path>` | Path | `.` | Root directory containing states |
| `--show <kind>` | `states\|transitions\|paths\|graph` | `paths` | What to list |

### clc missouri validate

Validate `missouri.yml` files without running.

```
clc missouri validate [-d <path>]
```

### clc missouri init

Initialize a new missouri project.

```
clc missouri init [-d <path>]
```

### clc missouri state add

Add a new state.

```
clc missouri state add <name> [-d <path>] [--from <state>]
```

| Flag | Type | Description |
|------|------|-------------|
| `-d, --dir <path>` | Path | Root directory containing states |
| `--from <state>` | String | Copy from an existing state and create a placeholder transition |

### clc missouri report

Generate a report from recorded runs.

```
clc missouri report [-d <path>] [--format <fmt>] [--run <id>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-d, --dir <path>` | Path | `.` | Root directory containing states |
| `--format <fmt>` | `terminal\|html\|md` | `terminal` | Report format |
| `--run <id>` | String | latest | Specific run ID to report on |

### clc missouri serve

Serve an HTML report locally.

```
clc missouri serve [-d <path>] [--run <id>] [--port <n>]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-d, --dir <path>` | Path | `.` | Root directory containing states |
| `--run <id>` | String | latest | Specific run ID to serve |
| `--port <n>` | u16 | `8080` | Port to serve on |
