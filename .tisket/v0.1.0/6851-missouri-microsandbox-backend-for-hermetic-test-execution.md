---
title: "Missouri microsandbox backend for hermetic test execution"
status: cancelled
priority: 2
assignee:
labels: [missouri, microsandbox]
depends_on: [8jyy-clc-builder-persistent-nix-builder-sandbox-management]
created: 2026-03-20T02:57:41Z
updated: "2026-03-23T00:08:51Z"
---

New `Backend` impl alongside `BareBackend` and `NixBackend`. Transitions
run inside a microsandbox microVM — full process, filesystem, and network
isolation per test.

## How it works

1. Boot sandbox from OCI image (built by the builder sandbox from flake.nix)
2. Mount test state directory into the VM via volume
3. Execute transition command, capture stdout/stderr
4. Compare filesystem state against target (same as existing backends)
5. Tear down sandbox

## Integration points

- `microsandbox-core` as library dep for sandbox lifecycle
- `NetworkScope::None` by default (hermetic) or `::Public` if test needs network
- `detect_sandbox()` extended: `SandboxConfig::Microsandbox` variant
- Parallel test paths get independent sandboxes (same as BareBackend with temp dirs)

## What this replaces

- `NixBackend`'s `nix shell` wrapping (packages are in the image)
- `BareBackend` for tests that want isolation
- The composable sandbox model tisket (VM provides all isolation layers at once)
