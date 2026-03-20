---
title: "Missouri builds OCI images from flake.nix devShell"
status: todo
priority: 2
assignee:
labels: [missouri, microsandbox]
depends_on: [8jyy-clc-builder-persistent-nix-builder-sandbox-management, 6851-missouri-microsandbox-backend-for-hermetic-test-execution]
created: "2026-03-20T02:57:41Z"
updated: "2026-03-20T02:57:41Z"
---

A `flake.nix` in a missouri state directory defines the test's execution
environment as a standard nix devShell. Missouri reads the devShell's
packages and builds an OCI image from them inside the builder sandbox.
The user never writes `dockerTools` or anything OCI-specific.

## User writes

```nix
# tests/missouri/dispatched/flake.nix
{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { nixpkgs, ... }: {
    devShells.aarch64-linux.default = nixpkgs.legacyPackages.aarch64-linux.mkShell {
      packages = with nixpkgs.legacyPackages.aarch64-linux; [
        python3 git mitmproxy
      ];
    };
  };
}
```

## Missouri does

Inside the builder sandbox, evaluates:
```nix
let
  shell = (builtins.getFlake "/path/to/state").devShells.aarch64-linux.default;
in dockerTools.buildLayeredImage {
  name = "missouri-test";
  contents = shell.buildInputs ++ shell.nativeBuildInputs;
}
```

The OCI tarball comes back via shared volume, loaded into microsandbox.

## Inheritance

No flake.nix in a state dir → look in parent dirs → fall back to
`packages:` list in missouri.yml → fall back to bare backend.

Same inheritance pattern as missouri.yml config.
