---
title: "AgentSpec: declarative agent config in clc-sdk with missouri agent eval"
status: done
priority:
assignee:
labels: [agent, missouri, clc-sdk]
depends_on: []
created: 2026-04-03T13:18:52Z
updated: "2026-04-03T15:07:23Z"
---

## Summary

Introduce AgentSpec as a declarative agent configuration struct in clc-sdk, then build missouri agent eval to use it for LLM-as-judge assertions.

## AgentSpec (clc-sdk)

YAML-serializable struct with named fields:
- model (optional, falls back to project default)
- max_turns (optional)
- max_cost_cents (optional)
- extra_args (escape hatch for unlisted CLI flags)

Two parse paths:
- from_markdown: parses YAML frontmatter, returns (AgentSpec, body)
- from_yaml: for embedding in other config files (tiskets, clc.yml)

Merge with AgentDefaults to produce an AgentConfig for dispatch.

## missouri agent eval <name>

Reads .missouri/<name>.md, parses frontmatter as AgentSpec, body becomes the agent prompt. Launches an agent session in the state directory, waits for verdict.

## missouri agent pass / fail <details>

Verdict protocol. The agent calls one of these to terminate the eval. pass exits 0, fail exits nonzero with details captured as the assertion error.

## Wire into missouri assertions

assertions:
  - agent: eval-foo
    name: "output meets quality bar"

## Future consumers (not in scope, but informed the design)

- Tiskets: agent field specifying the agent config for the assigned worker
- CoordinatorScope: replace standalone model field with embedded AgentSpec
- clc dispatch: read agent spec from tisket instead of --model flag
