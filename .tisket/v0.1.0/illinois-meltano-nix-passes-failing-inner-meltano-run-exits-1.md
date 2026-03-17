---
title: "illinois_meltano_nix_passes failing — inner meltano run exits 1"
status: in_progress
priority:
assignee:
labels: [missouri, bug]
depends_on: []
created: 2026-03-17T03:50:37Z
updated: "2026-03-17T21:33:33Z"
---

## Scratch Notes

### Investigation (2026-03-17)

**Root cause**: The `09-meltano` fixture uses an unpinned pip_url for tap-csv:
`pip_url: git+https://github.com/MeltanoLabs/tap-csv.git` (no tag/commit)

This pulls from HEAD of the MeltanoLabs/tap-csv main branch. The HEAD has changed
since the fixture was created (latest tag: v1.3.2, Nov 2025). When `meltano run`
auto-installs tap-csv from the unpinned URL, the newer HEAD code causes `meltano run`
to exit 1.

**Failing path**: `pipeline-ready → pipeline-ran` (independent root state, not part of
the setup path empty → ... → configured). The command `cd meltano-project && uv run
meltano run tap-csv target-jsonl` exits 1 due to tap-csv installation failure.

**Files consulted**:
- `missouri/tests/illinois.rs` — the failing test (`illinois_meltano_nix_passes`)
- `missouri/tests/fixtures/09-meltano/` — fixture structure
- `missouri/src/executor.rs` — NixBackend, env_clear(), build_path_env
- `missouri/src/config.rs`, `graph.rs` — mitmproxy addition (not the cause)
- `pipeline-ready/meltano-project/meltano.yml` — has unpinned pip_url
- `tap-csv--meltanolabs.lock` — also has unpinned pip_url

**Fix**: Pin tap-csv pip_url to `git+https://github.com/MeltanoLabs/tap-csv.git@v1.3.2`
in `pipeline-ready` and `pipeline-ran` states (both meltano.yml and lock files).
Do NOT change setup-path states (tap-added, target-added, configured) — those are
outputs of `meltano add tap-csv` which fetches from hub; changing them would break
state comparison.

**Test added**: `illinois_meltano_fixture_uses_pinned_pip_url` in illinois.rs

**Next**: Implement the fix, build, verify `illinois_meltano_nix_passes` passes.
