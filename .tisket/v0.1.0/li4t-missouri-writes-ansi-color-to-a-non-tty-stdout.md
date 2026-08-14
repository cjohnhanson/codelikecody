---
title: "missouri writes ANSI color to a non-tty stdout"
status: todo
priority:
assignee:
labels: [quality]
depends_on: []
created: 2026-08-14T20:01:09Z
updated: 2026-08-14T20:01:09Z
---

## Scratch Notes

Observed all session: missouri run | grep shows raw [32m escapes. A good citizen colors only when stdout is a tty and honors NO_COLOR. Check the other tools for the same behavior while fixing.
