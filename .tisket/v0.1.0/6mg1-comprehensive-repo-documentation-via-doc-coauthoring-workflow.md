---
title: "comprehensive repo documentation via doc-coauthoring workflow"
status: in_progress
priority: 1
assignee:
labels: [docs]
depends_on: []
created: 2026-04-03T23:44:14Z
updated: "2026-04-04T11:28:32Z"
---

## Problem

Someone looking at this repo can't understand what's going on. There are
13 crates in the workspace, a sophisticated orchestration system, a test
framework, a secret manager, a browser automation tool, a skill aggregator,
a knowledge base — and no coherent documentation that ties it together.

Existing docs are scattered: some bundled in binary (clc/docs/,
missouri/docs/), some in skills, some in CLAUDE.md, some only in
conversation histories and tisket descriptions. Principles and design
decisions live in people's heads (and chat transcripts), not in the repo.

## Scope

Every subproject. README through deep reference. The full inventory:

### Workspace-level
- README.md — what is this repo, what does it build, how to get started
- Architecture — how the crates relate, what depends on what
- Development guide — how to contribute, conventions, the phase system

### Per-subproject documentation
- clc — orchestration engine (supervisor, coordinators, workers, Docker,
  mTLS, phase guards, review gates, topology config, dispatch, git transfer)
- clc-sdk — coordination backend, agent traits, AgentSpec, workspace abstractions
- clc-api — HTTP API for the web frontend
- clc-web — Leptos web app, board view, issue detail
- tisket — issue tracker (repo format, frontmatter, status lifecycle, CLI)
- missouri — test framework (state graphs, transitions, assertions, services,
  agent evals, workspace mode, comparators, network interception)
- almanac — skill aggregator (skill format, sources, indexing, CLI)
- belmont — secret management (providers, scrubbing, workspace mode)
- moose — browser automation (CDP, navigation, screenshots, screencasts)
- zettel — knowledge base (note format, linking, search)
- mdstore — markdown document store
- claude-code — Claude Code protocol types and agent abstraction

### Cross-cutting
- clc.yaml topology format — full schema reference with examples
- Reviewer system — .clc/reviewers/ format, almanac skill references
- Workflow engine — phases, transitions, permissions, review gates
- Docker worker pipeline — image build, container lifecycle, certs, git transfer
- Permission model — phase guard, API grants, escalation chain

## Source materials

Everything is fair game for gathering context:
- Conversation histories (/search-history)
- Tisket histories (closed issues, scratch notes)
- Existing bundled docs and skills
- Git log and commit messages
- The code itself
- Design decisions captured in chat but never written down

## Approach

Use /doc-coauthoring for each major section:
1. Gather source materials (code, conversations, tiskets, existing docs)
2. Draft section-by-section — user directs, agent writes
3. Fresh-eyes review via subagent — can someone unfamiliar follow this?
4. docs-review evaluation (Diataxis type discipline, accuracy to code)
5. Revise and land

Diataxis types for each piece:
- Tutorials — getting started guides, first-time walkthroughs
- How-to — configuring clc.yaml, writing reviewers, building images
- Reference — CLI commands, config schemas, API endpoints
- Explanation — architecture, design rationale, why things are the way they are

## Done When

- A newcomer can clone the repo and understand what every crate does
  within 30 minutes of reading
- Every CLI tool has a complete command reference
- Every config file has a schema reference with examples
- Architecture docs explain the orchestration pipeline without requiring
  source code reading
- Design principles are documented — not just what but why
- Fresh-eyes subagent can answer questions about any subproject after
  reading only docs (not code)
- No documentation references stale APIs, removed features, or
  aspirational behavior that doesn't exist yet

## Scratch Notes
