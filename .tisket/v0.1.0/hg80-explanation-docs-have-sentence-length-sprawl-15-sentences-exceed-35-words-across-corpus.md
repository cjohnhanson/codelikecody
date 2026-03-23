---
title: "explanation docs have sentence-length sprawl — 15+ sentences exceed 35 words across corpus"
status: todo
priority:
assignee:
labels: [docs, writing-quality]
depends_on: []
created: "2026-03-23T03:12:25Z"
updated: "2026-03-23T03:12:25Z"
---

## Problem

Explanation docs should use concise sentences so agents and humans can parse them quickly. Several sentences across the what-is-*.md files exceed 35 words, forcing the reader to hold too much in working memory. Specific examples:

1. what-is-codelikecody.md line 29: "Trunk is read-only — the hooks block Edit, Write, and NotebookEdit entirely, and restrict Bash to a conservative allowlist (git, cargo, clc, missouri, tisket queries, and a handful of read-only utilities like ls, cat, find)." (42 words)

2. what-is-tisket.md line 37: "Tisket provides git-aware divergence detection (has the issue changed since this branch diverged?) and full-text search across titles, bodies, and scratch notes. The scratch notes section on each issue serves as the agent's working memory — the only persistent state that survives context compaction and session boundaries." (two sentences, 46 words total, functioning as a single unit)

3. what-is-codelikecody.md line 45: "Missouri handles the scaffolding — temp dirs, file comparison, path enumeration, parallel execution — so tests are just directories of expected output. The missouri getting started tutorial walks through building a test suite from scratch, and the CLI reference covers the full config schema." (43 words across two linked sentences)

## Open Questions

- What's the target maximum sentence length? 25 words? 30?
- Should this be addressed only in what-is pages, or across all explanation-type docs?
- Are there automated tools in the repo's writing evals that could enforce this?

## Why It Matters

Long sentences in agent-consumed documentation waste context window tokens on parsing overhead. When an agent reads a 40-word sentence, it's spending capacity on syntax that could go to reasoning.
