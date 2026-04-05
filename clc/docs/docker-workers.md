<!-- metadata
title: "Docker Workers"
description: "How Docker workspaces are built, launched, and connected"
type: explanation
-->

# Docker Workers

When a coordinator dispatches a worker with `workspace: docker`, the
worker runs inside a Docker container instead of a local git worktree.
The container gets its own copy of the codebase, its own SSH server,
and a reverse tunnel back to the supervisor API.

## Image

The worker image is built from `docker/worker/Dockerfile`. It's a
multi-stage build:

1. **Builder stage** — compiles the Rust workspace (clc, tisket,
   missouri, and other binaries)
2. **Runtime stage** (Debian bookworm-slim) — installs Node.js, npm,
   git, openssh-server, curl, and copies the compiled binaries plus
   the Rust toolchain (for in-container builds)

The runtime stage configures sshd with root login via public key
authentication and `GatewayPorts yes` (needed for reverse tunnels).
The container's default command is `/usr/sbin/sshd -D -e`.

Build the image before dispatching Docker workers:

```
docker build -t clc-worker -f docker/worker/Dockerfile .
```

## Container lifecycle

### Create

clc creates the container using the Bollard Docker SDK. The container
gets:

- The SSH public key mounted at `/root/.ssh/authorized_keys` (read-only)
- A random high port mapped to the container's port 22 (sshd)
- No project code — the codebase is transferred after start via git

### Code transfer

Once the container is running and sshd is accepting connections:

1. The host packs the `.git/` directory as a tar+gzip archive
2. The pack and a refs JSON file are transferred via SSH using
   `clc workspace write-file`
3. Inside the container, `clc workspace receive` extracts the pack,
   writes the git index, and checks out the target branch

The worker's git config sets author/committer to
`clc-worker@clc.local`.

### Communication

Docker workers communicate with the supervisor through two channels:

**Reverse SSH tunnel** — the SSH session sets up a reverse port
forward from `localhost:<tunnel_port>` inside the container back to
the supervisor's API port (19100) on the host. The container talks
to `https://localhost:<tunnel_port>` for phase queries, permission
requests, and output streaming.

**Supervisor API** — an Axum HTTP server at `0.0.0.0:19100` on the
host. Routes cover agent status, messages, permissions, phase state,
output collection, git pack transfer, stdin piping, tool checking,
dispatch status, and pickable tisket listing. Docker workers on the
same host can also reach it via `host.docker.internal:19100`.

Authentication uses bearer tokens passed via the `--api-url` flag.

### Agent startup

Inside the container:

1. `clc workspace init` creates `.clc/worker/` with named pipes,
   state files, and hooks
2. `clc workspace start` launches the Claude Code process with the
   branch, model, and API URL configured

The worker then proceeds through the normal phase workflow, making
API calls through the tunnel whenever it needs to check phase state,
request permissions, or report status.

### Cleanup

When work completes or the worker is stopped:

1. Coordinator calls `clc worker <id> stop`
2. Container is stopped via `docker.stop_container()`
3. Container is removed via `docker.remove_container(force=true)`
4. Completed work is landed (merged to trunk) before removal

## Configuration

In `clc.yml`:

```yaml
supervisor:
  coordinators:
    - id: backend
      workspace: docker
      image: clc-worker
      max_workers: 3
```

Or in the topology file (`clc.yaml`):

```yaml
workspaces:
  worker:
    isolation: docker
    docker_image: clc-worker
coordinators:
  backend:
    workspace: worker
    selector:
      label: backend
```
