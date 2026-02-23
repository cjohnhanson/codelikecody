---
title: "Missouri integration"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

clc runs missouri tests as a library dependency. Discover graph, enumerate paths,
execute, report results. Used by `clc done` to verify tests green, and by status
query to report missouri test status.

## Missouri tests

State: project-with-missouri (initialized clc + missouri test suite, some passing some failing)
Assertions:
- `clc` can discover and report missouri test status
- `clc` correctly distinguishes all-green vs has-failures
- `clc` handles missing missouri project gracefully (no tests/missouri/ dir)
