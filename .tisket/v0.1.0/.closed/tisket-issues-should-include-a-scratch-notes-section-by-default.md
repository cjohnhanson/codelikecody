---
title: "Tisket issues should include a Scratch Notes section by default"
status: done
priority:
assignee:
labels: [clc, tisket]
depends_on: []
created: 2026-03-01T16:56:52Z
updated: "2026-03-02T03:44:19Z"
---

When tisket creates an issue (or when clc pickup picks one up), the issue file should have a `## Scratch Notes` section at the bottom. This is working memory for agents — decisions, approaches tried, files consulted, next steps.

Currently the SessionStart hook tells workers to use scratch notes but the section doesn't exist in the issue file, so nobody writes to it.

Two possible insertion points:
1. `tisket issue create` adds the section when creating an issue
2. `clc pickup` adds the section if it's missing when picking up

Either way, the section should be present by the time a worker starts.
