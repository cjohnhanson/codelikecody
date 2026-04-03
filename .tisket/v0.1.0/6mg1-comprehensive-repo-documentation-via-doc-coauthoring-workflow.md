---
title: "comprehensive repo documentation via doc-coauthoring workflow"
status: todo
priority: 2
assignee:
labels: [docs]
depends_on: []
created: "2026-04-03T23:44:14Z"
updated: "2026-04-03T23:44:14Z"
---

## Problem

Someone looking at this repo for the first time can't understand what's
going on. The codebase has a sophisticated orchestration system (supervisor,
coordinators, workers, Docker workspaces, mTLS, phase guards, review gates,
topology config, named reviewers backed by almanac skills) — but the
documentation doesn't tell a coherent story. Existing docs are scattered
across bundled docs (`clc/docs/`), skills, CLAUDE.md, and tisket
descriptions.

## Approach

Use the `/doc-coauthoring` skill — structured co-authoring with
section-by-section drafting, fresh-eyes reader testing, and iterative
revision. This is human-directed work, not autonomous.

The Diataxis framework (from the `writing-docs-eval` skill) structures
what gets written:

### Tutorials (learning-oriented)
- Getting started: from clone to first `clc up` with workers dispatching
- Writing your first tisket and watching it get picked up

### How-to guides (task-oriented)
- Configuring `clc.yaml` — workspaces, coordinators, workflows, selectors
- Writing reviewers — AgentSpec frontmatter + almanac skill references
- Labeling tiskets for different coordinators
- Building the Docker worker image
- Adding a new workflow

### Reference (information-oriented)
- `clc.yaml` schema — all fields, types, defaults
- Reviewer file format
- Supervisor API endpoints
- Workflow phase graph and transition rules
- CLI command reference (clc up, clc dispatch, clc done, etc.)

### Explanation (understanding-oriented)
- Architecture: supervisor → coordinator → worker pipeline
- Why phases and guards exist
- Permission flow: phase guard → API grants → escalation → coordinator → human
- How git transfer works (tar of .git, import_pack, ff_merge)
- mTLS and agent identity

## Process

For each section:
1. Draft collaboratively via `/doc-coauthoring`
2. Run fresh-eyes review — does someone unfamiliar actually understand this?
3. Run `docs-review` reviewer criteria (Diataxis type discipline, accuracy)
4. Revise based on feedback
5. Land

## Done When

- A newcomer can go from clone to running `clc up` with working workers
  by following the tutorial
- The `clc.yaml` schema is fully documented with examples
- Architecture is explained clearly enough that the orchestration model
  makes sense without reading source code
- Fresh-eyes subagent can answer basic questions about the system after
  reading only the docs (not the code)
