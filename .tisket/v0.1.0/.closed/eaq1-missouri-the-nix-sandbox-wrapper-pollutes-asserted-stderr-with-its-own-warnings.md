---
title: "missouri: the nix sandbox wrapper pollutes asserted stderr with its own warnings"
status: done
priority: 2
assignee:
labels: [missouri, tests]
depends_on: []
created: 2026-08-13T00:55:10Z
updated: "2026-08-14T14:59:00Z"
---

The nix shell invocation prints 'warning: --no-registries is deprecated' into stderr. Suites that assert exact stderr (zettel: 6 assertions) fail on missouri's own wrapper output, not on the tool under test. missouri must keep its sandbox machinery out of the streams it compares: filter wrapper-origin lines before comparison, or update the deprecated flag, or both. Workaround verified in zettel: MISSOURI_SANDBOX=preinstalled makes the suite green.

## Scratch Notes

FIXED: the wrapper passed the deprecated --no-registries; nix 2.34 prints a deprecation warning on stderr that merges into every asserted stderr. executor.rs now passes --no-use-registries (silent, same semantics). Verified: tisket suite 27/27 in full nix mode; gaff 15, almanac 6, zettel 8, belmont 1 all pass in nix mode with the fixed binary. On retire-clc-config.
