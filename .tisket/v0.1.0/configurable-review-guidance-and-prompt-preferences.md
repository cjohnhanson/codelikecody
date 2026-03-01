---
title: "Configurable review guidance and prompt preferences"
status: todo
priority:
assignee:
labels: [clc]
depends_on: [review-phase-coordinator-gated-code-review-before-merge]
created: "2026-03-01T13:05:00Z"
updated: "2026-03-01T13:05:00Z"
---

Review worker prompting should be configurable per-project. The baseline review checks (Missouri test exhaustiveness, code quality, style) should be built in, but projects should be able to add or override review criteria.

This is part of a broader need: all prompt content in clc should have some room for configurability or preference. Review guidance is one instance of that.

Separate from the review phase implementation itself — this is about the content and customizability of the review prompt, not the workflow mechanics.
