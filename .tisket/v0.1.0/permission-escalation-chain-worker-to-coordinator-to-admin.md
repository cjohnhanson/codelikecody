---
title: "Permission escalation chain: worker to coordinator to admin"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-03T01:33:43Z
updated: "2026-03-03T01:35:07Z"
---

## Problem

Permission escalation currently goes worker → user directly. With coordinators as a middle layer, the chain should be worker → coordinator → admin. Coordinators should be able to handle routine permission requests (within their guidelines) without bothering the human.

## Design

### Three-tier chain

1. Worker hits a permission denial → files `clc permissions request "<description>"`
2. Coordinator sees the request (via `clc permissions list` or `clc worker <id> check`)
3. Coordinator decides:
   - **Grant** — if the request falls within its auto-grant policy → `clc permissions grant <worker-id> "<rule>"`
   - **Escalate** — if outside its policy → `clc permissions escalate <worker-id> "<description>"`
4. Escalation lands in `.clc/escalations/` visible to the admin session
5. Admin reviews via `clc permissions inbox`, grants or denies

### Changes from current system

The current escalate command writes to `.clc/escalations/` on the coordinator's working directory. For admin visibility, escalations need to be written somewhere the admin session can see — either on main (requires coordinator to commit to main, which we're trying to avoid) or in a shared location. Options:
- Coordinator pushes escalation files to its branch, admin reads from there
- Shared escalation directory outside of git
- Coordinator sends a message to admin session (if admin session has an inbound pipe)

## Depends on
- `coordinator-permission-guidelines-configurable-auto-grant-policy-per-coordinator`
- `admin-session-manage-coordinators-like-coordinators-manage-workers`
