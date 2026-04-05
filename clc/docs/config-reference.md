<!-- metadata
title: "Configuration Reference"
description: "Complete clc.yml schema with all fields, types, and defaults"
type: reference
-->

# Configuration Reference

clc reads configuration from the first file found:

1. `clc.yml` (project root)
2. `clc.toml` (project root)
3. `.clc/config.yml` (legacy)

All fields are optional. Missing fields use defaults.

## Top-level fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `main_branch` | string | `"main"` | Branch name for trunk |
| `admin_branch` | string | `"clc-admin"` | Branch for admin worktree |
| `required_attempts` | integer | `1` | Attempts before phase advance |
| `worker` | object | `{}` | Worker permission config |
| `coordinator` | object | `{}` | Coordinator decision config |
| `supervisor` | object | `{}` | Supervisor config |
| `workflows` | map | `{}` | Named workflow definitions |
| `rules` | list | `[]` | Policy rules for workflow selection |
| `skills` | list | `[]` | Almanac skill sources |

## worker

Controls baseline permissions for dispatched workers.

```yaml
worker:
  permissions:
    default:
      - "Read"
      - "Grep"
      - "Glob"
      - "Write({worktree}/**)"
    deny:
      - "Write({worktree}/.clc/**)"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `permissions.default` | list of strings | `[]` | Patterns granted at dispatch |
| `permissions.deny` | list of strings | `[]` | Patterns never granted |

Permission patterns use tool names with optional glob arguments:
`"Read"`, `"Edit(tests/**)"`, `"Bash(cargo *)"`.

## coordinator

Controls how coordinators handle permission requests.

```yaml
coordinator:
  auto_grant:
    - "Bash(cargo *)"
    - "Bash(npm *)"
  always_escalate:
    - "Bash(rm *)"
    - "Bash(git push *)"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_grant` | list of strings | `[]` | Patterns approved without judgment |
| `always_escalate` | list of strings | `[]` | Patterns forwarded to user |

Requests matching neither list are evaluated by the coordinator agent.

## supervisor

Controls the supervisor process and coordinator topology.

```yaml
supervisor:
  poll_interval: 10
  coordinators:
    - id: backend
      label: backend
      max_workers: 3
      model: opus
      workspace: docker
      image: clc-worker
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `poll_interval` | integer | `10` | Seconds between dispatch polls |
| `api_port` | integer | `19100` | Port for the supervisor API server |
| `tunnel_base_port` | integer | `19200` | Base port for SSH reverse tunnels |
| `coordinators` | list | `[]` | Coordinator scope definitions |

### Coordinator scope

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | string | required | Unique coordinator identifier |
| `project` | string | none | Filter to tisket project |
| `label` | string | none | Filter to issues with this label |
| `exclude_label` | string | none | Skip issues with this label |
| `max_workers` | integer | `3` | Concurrent worker limit |
| `model` | string | `"opus"` | Claude model for workers |
| `workspace` | string | `"worktree"` | Isolation type (opaque string) |
| `image` | string | none | Container image for SSH-based workspaces |
| `auto_grant` | list | `[]` | Per-coordinator auto-grant patterns |
| `always_escalate` | list | `[]` | Per-coordinator escalation patterns |
| `workflow` | string | none | Override workflow for this coordinator |

## workflows

Named workflow definitions. Each defines a directed graph of phases.

```yaml
workflows:
  tdd:
    description: "Test-driven development"
    phases:
      - name: tests-unwritten
        instructions: "Write failing tests."
        permissions:
          allow: ["Edit(tests/**)"]
          deny: ["Edit", "Write"]
        transitions:
          - target: tests-written
            review: test-review
      - name: tests-written
        transitions: [implementing]
      - name: implementing
        nudge: "Run tests to check progress."
        transitions: [green]
      - name: green
        can_stop: true
        transitions:
          - implementing
          - target: done
            review: [scope-check, code-review]
      - name: done
```

### WorkflowDef

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `description` | string | none | Human-readable description |
| `phases` | list | `[]` | Phase definitions |

### PhaseDef

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Phase identifier |
| `instructions` | string | none | Injected into agent context |
| `nudge` | string | none | Reminder after tool use |
| `can_stop` | boolean | `false` | Whether agent can exit |
| `permissions` | object | none | Allow/deny patterns |
| `transitions` | list | none | Valid next phases |

Phases can also be specified as plain strings for minimal definitions:

```yaml
phases: ["draft", "review", "done"]
```

### TransitionDef

Transitions can be simple (just a target name) or rich (with review
gates). Review gates name the reviewer agents from `.clc/reviewers/<name>.md`
that must approve before the transition is allowed.

```yaml
# simple — no review gate
transitions: [implementing, tests-unwritten]

# rich — reviewer agents must approve before advancing
transitions:
  - implementing
  - target: done
    review: [scope-check, code-review]

# single reviewer (string instead of list)
transitions:
  - target: tests-written
    review: test-review
```

| Field | Type | Description |
|-------|------|-------------|
| `target` | string | Target phase name |
| `review` | string or list | Reviewer agent names that must approve |

## rules

Policy rules select workflows based on issue metadata. Evaluated in
order; first match wins. Falls back to a workflow named `"default"` if
defined, then to the hardcoded TDD workflow.

```yaml
rules:
  - workflow: spike
    match:
      label: spike
  - workflow: docs-only
    match:
      label: docs
  - workflow: tdd
    match: {}
```

| Field | Type | Description |
|-------|------|-------------|
| `workflow` | string | Workflow name to use |
| `match.label` | string | Match issues with this label |
| `match.project` | string | Match issues in this project |
| `match.status` | string | Match issues with this status |

## skills

Almanac skill sources beyond built-ins.

```yaml
skills:
  - path: ./custom-skills
  - git: https://github.com/org/skills.git
```

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Local directory containing skills |
| `git` | string | Git repository URL (planned) |

## Reviewers

Reviewers are defined as markdown files in `.clc/reviewers/`, not in
`clc.yml`. Each file has AgentSpec frontmatter and a prompt body:

```
.clc/reviewers/
  code.md
  security.md
```

```markdown
---
model: sonnet
max_turns: 5
---

Check that tests are meaningful and cover edge cases.
Verify no obvious bugs or security issues.
```

Reviewer names correspond to the `requires` field in transitions.
A transition with `requires: [code]` triggers the reviewer defined
in `.clc/reviewers/code.md`.
