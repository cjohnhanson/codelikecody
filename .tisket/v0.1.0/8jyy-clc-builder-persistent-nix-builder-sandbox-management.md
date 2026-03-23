---
title: "clc builder: persistent nix builder sandbox management"
status: cancelled
priority: 2
assignee:
labels: [clc, microsandbox]
depends_on: []
created: 2026-03-20T02:57:41Z
updated: "2026-03-23T00:08:51Z"
---

`clc builder start/stop/status` manages a persistent microsandbox VM that
runs nix. Missouri and other tools use this sandbox to build OCI images
from flake.nix devShells. The /nix/store lives on a persistent host volume
so downloads and builds survive across stop/start cycles.

## Commands

- `clc builder start` — boot sandbox from configured image, mount nix store
  volume, bootstrap nix if first run
- `clc builder stop` — stop the VM, store volume persists
- `clc builder status` — running? store size? nixpkgs cached?

## Config (clc.yaml)

```yaml
builder:
  image: debian:bookworm-slim
  store: ~/.clc/nix-store
```

Image is user's choice — any Linux image with bash/curl/tar for nix bootstrap.

## Nix bootstrap inside the sandbox

On first start (empty store volume):
1. Download nix tarball, extract store paths
2. Configure: build-users-group=, sandbox=false, ssl-cert-file, experimental-features
3. Populate /etc/hosts (nix's bundled resolver can't resolve DNS in the VM)
4. Init nix db, load reginfo

Subsequent starts: nix store is warm, VM boots in <1s, ready immediately.

## Implementation

- `microsandbox-core` as library dep in clc (or shared workspace crate)
- Manages microsandbox server lifecycle (start on demand if not running)
- Sandbox name is deterministic (e.g., "clc-builder") so it persists across sessions

## Spike reference

See `use-microsandbox-for-hermetic-network-isolation-in-missouri-tests` branch,
`msb-spike` crate — validates nix running inside microsandbox, DNS workarounds,
SSL cert configuration, trivial builds succeeding.
