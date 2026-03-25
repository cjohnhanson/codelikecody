---
title: "Coordinators run on host even when workspace: docker is configured"
status: in_progress
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: 2026-03-25T03:27:39Z
updated: "2026-03-25T03:28:32Z"
---

## Problem

The coordinator scope in `clc.yml` accepts `workspace: docker`, but the
supervisor ignores it for the coordinator itself. `start_coordinator` in
`supervisor.rs:232-285` always spawns a local `clc coordinator-run`
process. The `workspace: docker` flag only gets passed through as a CLI
arg so the coordinator dispatches *workers* to docker — the coordinator
itself runs on the host.

This means coordinators have full host access: host filesystem, host
network, host docker socket. The isolation boundary only applies to
workers. A coordinator that misbehaves has the same blast radius as any
local process.

## Acceptance Criteria

- [ ] Given a coordinator scope with `workspace: docker`, when the
      supervisor starts it, then the coordinator runs inside a docker
      container via `SSHWorkspace` + `DockerEnvironment`, not as a local
      process
- [ ] Given a coordinator running in docker, when it needs to dispatch
      a worker, then it calls `POST /dispatch` on the supervisor API
      instead of spawning local processes
- [ ] Given `SSHWorkspace`, when starting an agent, then the start
      command is parameterized — not hardcoded to `clc workspace start`
- [ ] Given a coordinator in docker, when it talks to the supervisor,
      then it uses the reverse tunnel + mTLS, same as workers

## Done When

- `clc up` with `workspace: docker` on a coordinator scope starts the
  coordinator in a container
- Coordinator dispatches workers via supervisor API, not local process
  spawning
- `SSHWorkspace` accepts a configurable start command
- Coordinator in docker has no host filesystem access (no mounts)

## Scratch Notes

### Design

**Coordinator in Docker runs `clc coordinator-run` instead of `clc workspace start`.**

SSHWorkspace currently hardcodes `clc workspace start` as the agent command.
The start command needs to be parameterized — coordinators run `clc coordinator-run --id <id> ...`
while workers run `clc workspace start --branch <id> ...`.

**Coordinator dispatches via API, not local process spawning.**

When `CLC_API_URL` is set (coordinator in Docker), `dispatch_with_workspace()` calls
`POST /dispatch` on the supervisor API. The supervisor creates the workspace and spawns the worker.
Coordinator in Docker has no Docker socket — it delegates workspace creation to the supervisor.

**New supervisor API endpoint: `POST /dispatch`**

Accepts: `{ tisket_id, model, coordinator_id }`. Supervisor creates the workspace
(Docker or worktree) and starts the worker. Returns the worker ID.

**Changes needed:**
1. `SSHWorkspaceConfig` gets a `start_command: Vec<String>` field (replaces hardcoded `clc workspace start`)
2. `supervisor.rs::start_coordinator()` — when workspace=docker, use SSHWorkspace instead of local spawn
3. `supervisor_api.rs` — add `POST /dispatch` endpoint
4. `coordinator_loop.rs` — when `CLC_API_URL` is set, dispatch via API instead of local
5. `dispatch.rs` — add `dispatch_via_api()` function
