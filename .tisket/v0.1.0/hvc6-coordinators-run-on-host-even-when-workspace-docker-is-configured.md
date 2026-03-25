---
title: "Coordinators run on host even when workspace: docker is configured"
status: todo
priority: 2
assignee:
labels: [clc, architecture]
depends_on: []
created: "2026-03-25T03:27:39Z"
updated: "2026-03-25T03:27:39Z"
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
