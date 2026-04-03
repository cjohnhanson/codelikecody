---
title: "nono sandbox: evaluate as kernel-enforced sandbox for Docker workers"
status: discovery
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-04-03T12:22:32Z
updated: "2026-04-03T12:22:51Z"
---

## Context

`clc up` currently runs workers in Docker containers with sshd. Docker provides
process and filesystem isolation but is heavyweight — each worker spins up a
full container, SSH connection, git pack transfer. Network isolation relies on
Docker networking (workers can reach the internet and each other).

[nono](https://nono.sh) is a kernel-enforced sandbox CLI and Rust library.
It uses Landlock (Linux 5.13+) and Seatbelt (macOS 10.5+) to apply
irreversible capability-based restrictions to a process. Once applied,
the sandboxed process cannot escalate permissions.

Key capabilities from the docs:
- **Filesystem**: `allow_read("/path")`, `allow_write("/path")` — granular
  path-level control. Everything else is denied.
- **Network**: Can block all network or allow specific destinations.
- **Credential injection**: Managed secret passing into sandboxed processes.
- **Atomic rollback**: Filesystem state recovery for unattended sessions.
- **Audit trail**: Cryptographic immutable provenance chain.
- **Profiles**: Pre-built configs for Claude Code, Codex, etc.
- **Detach/attach**: tmux-like session management.

Rust core library API:
```rust
let mut caps = CapabilitySet::new();
caps.allow_read("/data/models")?;
caps.allow_write("/tmp/workspace")?;
Sandbox::apply(&caps)?;
```

CLI: `brew install always-further/tap/nono`
Source: https://github.com/always-further/nono (Apache 2.0)

## What to evaluate

1. **Replace Docker with nono for local workers.** Instead of Docker containers,
   run `nono run --profile claude-code --allow-cwd -- clc workspace start ...`
   directly on the host. The worker gets a sandboxed process with filesystem
   restrictions (only its worktree + /tmp) and network restrictions (only the
   supervisor API). No Docker, no SSH, no pack transfer — just a worktree +
   nono sandbox.

2. **Rust library integration.** nono exposes a Rust core library
   (`CapabilitySet` + `Sandbox::apply()`). Could integrate directly into `clc`
   so the supervisor applies the sandbox before spawning the worker process.
   No CLI wrapper needed.

3. **Network policy.** Can nono restrict a worker to only talk to the supervisor
   API (localhost:19100) and block everything else? This would replace the mTLS
   grant system for network-level isolation.

4. **Rollback for failed workers.** nono's `--rollback` flag could replace the
   manual worktree cleanup on worker failure.

5. **macOS support.** Docker Desktop is the current path for macOS. nono uses
   Seatbelt on macOS — does this work well enough for the worker use case?

## What this is NOT

Not replacing Docker for CI or remote execution. Docker is still the right
choice for running on remote machines, cloud builders, etc. This is about
the local `clc up` developer experience — faster worker startup, simpler
architecture, same security guarantees.

## Done when

- Spike: install nono, run a Claude Code session inside it with filesystem
  and network restrictions matching what a worker needs
- Assess: does Landlock/Seatbelt provide equivalent isolation to Docker for
  the worker use case?
- Prototype: if viable, add a `WorkspaceType::Nono` variant alongside
  `Worktree` and `Docker`

## Scratch Notes
