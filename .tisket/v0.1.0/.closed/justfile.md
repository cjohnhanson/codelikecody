---
title: "Justfile with minimal task recipes"
status: done
priority:
assignee:
labels: [admin]
depends_on: []
created: 2026-02-26T06:00:00Z
updated: "2026-03-23T02:12:21Z"
---

Add a justfile with minimal recipes and add `just` to the flake.nix devShell.

## Recipes

```just
# Run all tests (cargo tests + missouri)
test:
    cargo test --workspace
    MISSOURI_SANDBOX=preinstalled clc missouri run

# Run only cargo tests
test-cargo:
    cargo test --workspace

# Run only missouri tests
test-missouri:
    MISSOURI_SANDBOX=preinstalled clc missouri run

# Serve docs site locally (placeholder until docs-site lands)
serve-docs:
    @echo "docs site not yet scaffolded"
```

Keep it minimal. More recipes get added as features land (e.g., `serve-docs` becomes real when the Leptos site exists).

## flake.nix change

Add `just` to `devShells.default.buildInputs`:

```nix
buildInputs = [
  pkgs.rust-bin.stable.latest.default
  pkgs.git
  pkgs.jq
  pkgs.just  # <-- add this
]
```

## Files

- `justfile` (new, repo root)
- `flake.nix` (add `pkgs.just` to devShell)
