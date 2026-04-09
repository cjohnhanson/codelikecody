---
title: "clc coordinators / clc workers admin CLI returns empty while supervisor has running instances"
status: in_progress
priority:
assignee:
labels: [clc, admin, bug]
depends_on: []
created: 2026-04-09T03:21:04Z
updated: "2026-04-09T13:10:45Z"
---

## Problem

While `clc up` is running with multiple coordinators and workers active (visible in `docker ps` and in the supervisor log), the admin CLI commands report empty:

```
$ clc coordinators
no coordinators

$ clc workers
no workers
```

But the supervisor log clearly shows:

```
supervisor started (2 coordinator(s), poll every 10s)
supervisor: coordinator 'auto' started in Docker
supervisor: coordinator 'standard' started in Docker
supervisor: worker '1fkl-...' started in Docker
```

And `docker ps` shows 17 running containers from `clc-worker:latest`.

## Root cause (hypothesis)

The admin CLI reads from a local state source (probably `.clc/state` files or a local SQLite), while the live supervisor tracks state via the coordination DB + API. When the supervisor runs in a terminal other than where `clc coordinators` is invoked, the admin CLI can't see the live state.

## Proposed fix

The admin CLI should query the supervisor API (`GET /coordinators`, `GET /workers`) when one is running. Fall back to the local state source only when no supervisor is reachable.

## Acceptance criteria

- While clc up is running: `clc coordinators` shows the active coordinators; `clc workers` shows the active workers
- When no supervisor is running: the commands report either 'no supervisor running' or fall back to last-known state

## Scratch Notes
