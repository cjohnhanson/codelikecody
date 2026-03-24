---
title: "belmont: workspace mode — resolve secrets via supervisor API over mTLS"
status: discovery
priority: 2
assignee:
labels: [belmont, supervisor, architecture]
depends_on: []
created: 2026-03-24T13:09:27Z
updated: "2026-03-24T13:09:37Z"
---

## Problem

Belmont v1 resolves secrets locally — the process running `belmont run` needs
direct access to backends (keyring, env, age). This works for human sessions
but breaks in workspace mode, where workers run in worktrees or Docker
containers that don't have the human's keychain or age identity.

## Design

Follows the same pattern as Kubernetes secrets: the supervisor is the central
authority that holds backend credentials and resolves secret values on behalf
of workspaces.

### Architecture (parallels k8s)

| | Kubernetes | Belmont |
|---|---|---|
| Central authority | API server + etcd | Supervisor + keyring/age |
| Node/worker | Kubelet | Workspace |
| Trust bootstrap | kubeadm join token → client cert | Join token → client cert |
| Transport | mTLS | mTLS (SSH reverse tunnel for SSH workspaces) |
| Access control | RBAC per namespace/secret | Could scope per workspace (future) |
| Injection | Env vars or volume mounts | Env vars via PTY |

Key difference from k8s: the supervisor doesn't store values in its own
database. It resolves on demand from backends (keyring, age, env on the
supervisor's machine). No at-rest storage concern on the supervisor side.

### Trust bootstrap

Same as the supervisor API join flow (landing in separate work):

1. Supervisor mints a join token
2. Workspace presents the token to the join endpoint
3. Supervisor issues a client cert for mTLS
4. From then on, the workspace authenticates via its client cert

The cert is the identity. No separate auth mechanism needed.

### Secret resolution in workspace mode

1. `belmont run` in a workspace reads `belmont.yml` for secret names
2. Instead of resolving `ref+` URIs locally, it calls the supervisor API:
   `POST /secrets/resolve` with the list of secret names
3. Supervisor verifies the workspace's mTLS client cert
4. Supervisor resolves each secret from its local backends (keyring, age, env)
5. Supervisor returns name/value pairs over the mTLS connection
6. Workspace injects values into the subprocess environment, scrubs output

### What stays the same

- `belmont.yml` format — same file, same `ref+` URIs, shared between
  supervisor and workspaces
- Prime text and agent experience — still sees `belmont://` references,
  still uses `belmont run`, resolution path is invisible
- Scrubbing — still happens locally in the workspace's PTY
- `belmont check` — works in workspace mode by calling the supervisor
  to verify resolvability without returning values

### What changes

- `belmont run` / registry resolution needs a mode switch: if connected to
  a supervisor, call the API; if standalone, resolve locally
- Supervisor needs a secrets endpoint authenticated by mTLS
- Connection detection: belmont needs to know whether it's in a workspace
  (check for mTLS cert/supervisor URL in `.clc/` state)

### Security considerations

- Secret values transit the mTLS connection. The connection is encrypted
  and mutually authenticated, but values exist in memory on both sides.
- The supervisor becomes a high-value target — compromise it and all
  workspace secrets are accessible. Same as k8s API server.
- Per-workspace access scoping (which secrets a specific workspace can
  request) is a future concern, not v1.

### Dependencies

Depends on the supervisor API mTLS and join-token work landing first.
This is additive to that infrastructure.
