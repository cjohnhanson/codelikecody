---
title: "illinois_meltano_nix_passes test failing"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-19T21:35:45Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

The `illinois_meltano_nix_passes` test should verify that missouri can run a meltano-based fixture end-to-end using nix-provided tools — it's an integration test proving that missouri's nix shell support works with a real data pipeline tool.

The test (`missouri/tests/illinois.rs`, line 345) uses `setup_illinois_nix_scenario("09-meltano", 0)` which copies the `09-meltano` fixture, sets up an illinois meta-test harness, and expects missouri to exit 0 (pass). The fixture at `missouri/tests/fixtures/09-meltano/` contains multiple states (`empty`, `configured`, `meltano-initialized`, `tap-added`, `target-added`, `pipeline-ready`, `pipeline-ran`) representing a meltano project lifecycle. The test is currently failing.

A prior closed issue (`a858`) fixed GNU sed syntax incompatibility on macOS. Another closed issue addressed unpinned pip URLs. The test now has a pinned pip_url check (`illinois_meltano_fixture_uses_pinned_pip_url`). The current failure may be a different root cause — possibly nix package resolution, meltano version changes, or environment variable issues in the illinois harness.

## Open Questions

- What is the actual failure output? The test needs to be run with `cargo test illinois_meltano_nix_passes -- --nocapture` to capture stdout/stderr.
- Is the meltano nix package still resolving to a compatible version, or has the nixpkgs channel moved past what the fixture expects?
- Is this a macOS-specific failure (environment differences), or does it also fail on Linux?

## Why It Matters

Illinois meta-tests are the primary verification that missouri can orchestrate real-world tool workflows. A persistently failing meltano test means the nix integration path is unverified and may be broken for users relying on meltano fixtures.
