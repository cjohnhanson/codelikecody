---
title: "Tisket integration"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

clc reads and writes tisket state as a library dependency. Open a repo, list
issues, read issue details, change status, close/reopen. Prerequisite: tisket
needs a `src/lib.rs` exposing its public API (work in the tisket repo).

## Missouri tests

State: project-with-tiskets (initialized clc + tisket repo with issues in various states)
Assertions:
- `clc` can list todo tiskets (reads from .tisket/)
- `clc` can read a specific tisket's details
- `clc` can change a tisket's status (verified by re-reading the file)
- `clc` handles missing tisket repo gracefully
