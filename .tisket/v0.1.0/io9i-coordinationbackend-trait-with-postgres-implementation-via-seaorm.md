---
title: "CoordinationBackend trait with Postgres implementation via SeaORM"
status: todo
priority: 2
assignee:
labels: [clc, architecture]
depends_on: [am9k-agent-trait-extract-claude-specific-code-from-workspace-into-agent-abstraction]
created: 2026-03-22T22:02:06Z
updated: "2026-03-22T22:03:30Z"
---

Replace filesystem communication (stdout.jsonl, pid files, stdin pipes, outbox) with a CoordinationBackend trait. First impl: Postgres via SeaORM.

The backend is the communication bus for the entire system. Message types: PermissionRequest, PermissionGrant, ReviewRequest, ReviewResult, StatusUpdate, AgentOutput, AgentInput.

Every interaction between agents goes through the backend: worker→coordinator permission requests, coordinator→admin escalations, review workflows, status updates. Database makes everything queryable, durable, and observable.

clc-api becomes the API layer over the backend. clc-web becomes the UI. Permission requests that escalate to humans show up in the UI.

Depends on: am9k (Agent trait)
