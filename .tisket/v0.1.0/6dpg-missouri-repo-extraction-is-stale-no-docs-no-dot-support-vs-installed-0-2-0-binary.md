---
title: "missouri repo extraction is stale — no docs/, no dot- support vs installed 0.2.0 binary"
status: todo
priority: 3
assignee:
labels: [missouri, infra]
depends_on: []
created: "2026-08-12T16:52:07Z"
updated: "2026-08-12T16:52:07Z"
---

During gaff test-design review, a reviewer compared ~/Projects/missouri (the extracted standalone repo) against the installed missouri 0.2.0 nix binary: the repo checkout has no docs/ directory and no dot- restoration support in source, both of which the installed binary has. The extraction from codelikecody appears incomplete or stale relative to the workspace member that still lives here. Reconcile: either re-extract from the current workspace member or delete the stale checkout until the zs98 extraction reaches missouri.
