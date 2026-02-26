---
title: "Workflow for pure test-improvement tasks"
status: todo
priority: 3
assignee:
labels: [clc]
depends_on: []
created: "2026-02-26T13:28:00Z"
updated: "2026-02-26T13:28:00Z"
---

The current phase system (tests-unwritten → tests-written → red → implementing →
green → done) assumes implementation work: write tests first, then implement. But
some tasks are purely about improving test coverage or test quality — adding
assertions, restructuring test states, improving test fidelity.

Questions to answer:
- Does the current phase system work for test-only tasks, or does it create
  friction?
- Should there be a separate phase progression for test work?
- Or is the existing system fine if you just skip through phases quickly?
- What about tasks that are purely about missouri test structure (adding states,
  transitions, reorganizing the graph)?

The guard enforces that non-test edits are blocked until the implementing phase.
For test-only work, all edits ARE test edits, so the restriction doesn't bite.
But the phase ceremony (advancing through 5 phases) may be unnecessary overhead
for a task that's "make these tests better."
