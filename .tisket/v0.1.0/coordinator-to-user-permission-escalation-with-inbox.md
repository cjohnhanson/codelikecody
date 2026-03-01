---
title: "Coordinator-to-user permission escalation with inbox"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-01T22:10:12Z"
updated: "2026-03-01T22:10:12Z"
---

Workers escalate to the coordinator via `clc permissions request`. The coordinator can grant or deny+redirect. But sometimes the coordinator itself needs to escalate to the user — when a permission decision is beyond its authority or judgment.

The escalation chain: worker → coordinator → user.

Needs:
- `clc permissions escalate "<description>"` — coordinator files a request for the user
- `clc permissions inbox` — user views pending coordinator escalations
- `clc permissions grant coordinator "<permission>"` — user grants to coordinator (who then passes it through to the worker)
- Or the user grants directly to the worker: `clc permissions grant <worker-id> "<permission>"`

The coordinator should escalate rather than grant when:
- The permission is broad or dangerous (e.g., network access, system commands)
- The coordinator is unsure whether the permission is appropriate
- The worker's request is ambiguous or unusual
