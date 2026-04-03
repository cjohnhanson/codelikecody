---
title: "Workspace configuration from clc.yaml"
status: done
priority:
assignee:
labels: [agents]
depends_on: []
created: 2026-03-11T02:19:54Z
updated: "2026-04-03T18:31:54Z"
---

Coordinator reads workspace type and agent from clc.yaml instead of hardcoding worktree + claude-code. WorkspaceConfig gets populated from the config file. Different workspace sections in clc.yaml produce different Workspace trait implementations.

Depends on: clc.yaml schema, oc3u (wire workspace trait)
Blocks: clc up
