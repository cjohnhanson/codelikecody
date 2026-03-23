---
title: "Docker workspace: Workspace trait implementation using Docker containers via bollard"
status: discovery
priority: 3
assignee:
labels: [clc, architecture]
depends_on: [am9k-agent-trait-extract-claude-specific-code-from-workspace-into-agent-abstraction, io9i-coordinationbackend-trait-with-postgres-implementation-via-seaorm, f1dm-loop-wrapper-replace-stop-hooks-with-resume-loop-lifecycle-management]
created: 2026-03-22T22:02:27Z
updated: "2026-03-23T02:12:47Z"
---

## Problem

The Workspace trait (`clc-sdk/src/workspace.rs`) should support multiple backend implementations — the trait design explicitly anticipates "future backends (Docker, Coder, K8s)" that "swap in without changing the coordinator loop."

Currently the only Workspace implementation is `WorktreeWorkspace` (`clc/src/workspace.rs`), which couples to git worktrees and local Claude Code processes via FIFO pipes. There is no Docker-based implementation. The bollard crate (a Rust Docker API client) appears in `Cargo.lock` as a dependency of microsandbox but is not used directly by clc or clc-sdk for workspace management. No code in the repository implements the `Workspace` trait methods (`start`, `send_message`, `recv_output`, `stop`) using Docker containers.

Without a container-based workspace, workers share the host filesystem and process namespace. There's no isolation between workers, no resource limits, and no way to run workers on remote hosts — the coordinator is limited to local worktree-based execution.

## Open Questions

- Should the Docker workspace use bollard directly, or go through a higher-level abstraction?
- How does container networking interact with the FIFO-based stdin/stdout protocol — does the workspace trait need to evolve to support network-based communication?
- What's the container image strategy — prebuilt images with clc/tisket/missouri baked in, or mount the host binaries?
- How does this interact with the Agent trait extraction (dependency: am9k) — does the agent need to be container-aware, or does the workspace abstract that away?

## Why It Matters

Local worktree execution is inherently limited: no isolation, no scalability beyond the host machine, no resource controls. A Docker workspace backend is the necessary step toward running workers on remote infrastructure and enforcing hermetic execution environments.
