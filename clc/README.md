# clc

Code like Cody. Opinionated workflow enforcement for coding agents.

clc is the workflow engine. It picks up tisket issues, creates isolated
workspaces, and enforces a phase system that mechanically constrains what
agents can do at each stage of test-driven development. Optionally, it
orchestrates multiple agents working in parallel through a
supervisor/coordinator/worker hierarchy.

## Workspaces

Workspace isolation is pluggable. Agents work in isolated environments
created per issue. Git worktrees and Docker containers are the current
backends. Each workspace gets its own branch,
its own phase state, and its own copy of the codebase.

## Phase system

Workflows define a directed graph of phases. Each phase declares what
files can be edited, what instructions the agent receives, and whether
the agent is allowed to stop. Constraints are enforced by intercepting
every tool call through Claude Code hooks, rejecting disallowed
operations before they reach the filesystem.

The default workflow is a TDD sequence:

`tests-unwritten` → `tests-written` → `red` → `implementing` → `green`
→ `review-requested` → `in-review` → `reviewed` → `done`

Custom workflows can define any phase graph. Policy rules in `clc.yml`
select workflows based on issue labels or project, so different kinds of
work can follow different processes. Phases support review gates
(require approval before advancing), configurable permissions (allow/deny
patterns per phase), and stop gates (which phases allow the agent to
exit).

## Orchestration

For multi-agent work, clc runs a three-tier hierarchy:

- **Supervisor** (`clc up`) — spawns and monitors coordinators
- **Coordinators** — poll for pickable tiskets, dispatch workers, handle
  permission requests, land completed work
- **Workers** — one agent per tisket, running in an isolated workspace,
  constrained by the phase system

Workers that need capabilities outside their defaults request permission. Coordinators grant, deny, or escalate to a human. The
permission system is configurable: auto-grant patterns for safe
operations, always-escalate patterns for dangerous ones.

Topology is declared in `clc.yml`:

```yaml
workspaces:
  worker:
    type: docker
    image: clc-worker
coordinators:
  backend:
    workspace: worker
    selector:
      label: backend
```

## Usage

```
clc init                    # set up clc in a project
clc pickup <issue-id>       # create workspace, start phase workflow
clc status                  # current phase, tisket, test results
clc status set <phase>      # advance or retreat
clc done                    # finalize and close tisket
clc merge <id>              # merge completed work to trunk

clc up                      # start supervisor
clc dispatch <id>           # spawn a worker for one tisket
clc workers                 # list active workers
clc worker <id> check       # worker status
clc worker <id> log         # worker output
clc worker <id> send <msg>  # send message to worker

clc docs [topic]            # bundled documentation
```

## Documentation

- [What is codelikecody?](docs/what-is-codelikecody.md) — philosophy and design
- [Getting Started](docs/getting-started.md) — first pickup-to-done walkthrough
- [Phase System](docs/phase-system.md) — phases, transitions, workflow definitions
- [Permission Model](docs/permissions.md) — guards, requests, grants, escalation
- [Orchestration](docs/orchestration.md) — supervisor, coordinators, workers
- [Configuration Reference](docs/config-reference.md) — complete clc.yml schema
- [CLI Reference](docs/cli-reference.md) — complete command documentation
