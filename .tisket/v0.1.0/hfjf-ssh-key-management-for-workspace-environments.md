---
title: "SSH key management for workspace environments"
status: discovery
priority: 3
assignee:
labels: [workspace, security]
depends_on: []
created: "2026-03-24T00:59:14Z"
updated: "2026-03-24T00:59:14Z"
---

Workspaces communicate over SSH. Currently using the user's existing
SSH keys (~/.ssh/). This works for local Docker containers and trusted
remote hosts but doesn't scale to:

- **Ephemeral environments**: Docker containers created per-dispatch
  should have per-container key pairs that are generated on create and
  destroyed with the container. The user's keys shouldn't be inside
  untrusted containers.

- **Multi-user**: multiple humans running `clc up` against the same
  Postgres-backed coordination DB, each with their own key identity.

- **Key rotation**: long-running environments should rotate keys
  periodically.

- **Audit**: which key was used for which workspace, when.

## Questions to scope

1. Should Docker environments use ephemeral keys (generated per-container)
   or the user's keys mounted read-only?
2. Should the coordinator have its own key pair separate from workers?
3. How are keys distributed to remote (non-Docker) environments?
4. Does gix SSH transport use the same keys as russh, or do they need
   separate configuration?
5. What's the threat model — are we protecting against a compromised
   worker container, or just avoiding key sprawl?
