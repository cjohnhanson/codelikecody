---
title: "Coordinator permission guidelines: configurable auto-grant policy per coordinator"
status: in_progress
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:44Z
updated: "2026-03-03T04:15:22Z"
---

## Problem

Coordinators need to know what permissions they can grant autonomously vs. what must be escalated. Currently there's no policy — the coordinator prompt just says "if it seems dangerous, escalate." That's too vague for reliable autonomous operation.

## Design

Permission guidelines at dispatch time and/or in config:

### Dispatch-time flags
- `clc coordinate --auto-grant "Bash(cargo *)" --auto-grant "Bash(npm *)"` — coordinator can grant these patterns to any worker
- `clc coordinate --escalate-all` — coordinator escalates everything (conservative mode)
- `clc coordinate --grant-config ./permissions-policy.yml` — external policy file

### Config-level (`.clc/config.yml`)
```yaml
coordinator:
  auto_grant:
    - "Bash(cargo *)"
    - "Bash(npm *)"
    - "Bash(just *)"
  always_escalate:
    - "Bash(rm *)"
    - "Bash(git push *)"
```

### Decision logic in coordinator prompt
The coordinator's system prompt includes its policy. When a worker requests a permission:
1. Check if it matches an `auto_grant` pattern → grant immediately
2. Check if it matches an `always_escalate` pattern → escalate immediately
3. Otherwise → coordinator uses judgment (with bias toward escalation)
