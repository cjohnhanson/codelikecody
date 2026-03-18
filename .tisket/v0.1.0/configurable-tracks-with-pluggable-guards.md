---
title: "Configurable tracks with pluggable guards"
status: discovery
priority:
assignee:
labels: []
depends_on: [switch-clc-config-to-yaml-as-primary-format]
created: "2026-03-18T03:02:40Z"
updated: "2026-03-18T03:02:40Z"
---

## Problem

clc's phase system is hardcoded to a single TDD pipeline (tests-unwritten → ... → done). Every tisket goes through the same phases with the same guards. This doesn't work for non-coding work — research tasks, planning, purchasing decisions — where the workflow is fundamentally different. The org repo has 60+ tiskets that are research/life tasks, not implementation work.

## Design

### Tracks

A **track** is a named, ordered phase pipeline defined in `clc.yaml`. Each project can define multiple tracks. A tisket is assigned to a track via **selectors** — tisket filter expressions evaluated in order, first match wins.

```yaml
tracks:
  research:
    select:
      labels: [research, planning, home-repair]
    phases: [researching, draft, done]

  coding:
    default: true
    phases: [tests-unwritten, tests-written, red, implementing, green, review-requested, in-review, reviewed, done]
```

- Selectors use the same filtering system as `tisket issue list`
- Ordering in the config determines precedence — first matching track wins
- Exactly one track must be marked `default: true` — catch-all for unmatched tiskets
- The current hardcoded TDD pipeline becomes the built-in default `coding` track

### Guards

Each phase in a track declares **guards** — constraints that gate tool use, session exit, and phase advancement. Guards are pluggable. Guard types:

1. **`allow`** — path glob restrictions on file-targeting tools (clc built-in)
2. **`tools`** — tool allowlist (clc built-in)
3. **`stop`** — whether the agent can exit the session (clc built-in)
4. **`nudge`** — post-tool-use message injected after edits (clc built-in)
5. **`missouri`** — test suite must pass (missouri-provided)
6. **`shell`** — run a command, nonzero exit = blocked
7. **`agent`** — spin up a subagent that writes a structured verdict to an outbox

YAML anchors allow reusing guard definitions across phases:

```yaml
guards:
  read-only: &read-only
    allow: [tests/missouri/**]
    tools: [Read, Grep, Glob]
    stop: false

tracks:
  coding:
    default: true
    phases:
      tests-unwritten:
        <<: *read-only
      tests-written:
        <<: *read-only
      implementing:
        allow: ["**"]
        stop: false
        nudge: "run tests before advancing"
      green:
        missouri: pass
        stop: true
```

### Agent guards

Agent guards use the existing inbox/outbox messaging system. The guard runner:

1. Spins up a subagent with the configured prompt + an outbox path
2. Subagent evaluates the condition and writes a structured result file to the outbox
3. Guard runner reads the result

Result schema:

```yaml
pass: bool
reasoning: string
```

The agent runtime is provider-agnostic — whatever agent provider the project is configured to use handles execution. clc doesn't assume Claude.

### Research track (motivating example)

```yaml
tracks:
  research:
    select:
      labels: [research, planning]
    phases:
      researching:
        tools: [WebSearch, WebFetch, Read, Grep, Glob]
        allow: [".tisket/**"]
        stop: false
      draft:
        allow: [".tisket/**"]
        stop: true
        agent:
          prompt: "Does this tisket contain substantive findings with sources cited and options compared?"
      done: {}
```

The tisket is the artifact — all findings, options, and recommendations are written back into the tisket markdown. No PR.
