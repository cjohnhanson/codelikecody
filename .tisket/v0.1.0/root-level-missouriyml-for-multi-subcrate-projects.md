---
title: "Root-level missouri.yml for multi-subcrate projects"
status: backlog
priority: 2
assignee:
labels: [missouri]
depends_on: []
created: "2026-02-26T04:24:26Z"
updated: "2026-02-26T04:24:26Z"
---

This repo has missouri test suites in subcrates (`tisket/tests/missouri/`,
`clc/tests/missouri/`) but nothing at the root. The clc reinforcement hook
reports "missouri: no tests" because it looks at the worktree/project root
and finds no `.missouri/` directory.

A root-level `missouri.yml` (or `.missouri/missouri.yml`) should know about
the subcrate test suites and aggregate them. `missouri run` from the root
should run all of them and report a unified result.

This also solves the reinforcement gap — once the root has a missouri config,
the hook detects it and the status line reflects actual test state instead of
"no tests."
