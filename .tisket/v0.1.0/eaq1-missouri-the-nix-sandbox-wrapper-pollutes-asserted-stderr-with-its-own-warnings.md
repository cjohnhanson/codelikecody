---
title: "missouri: the nix sandbox wrapper pollutes asserted stderr with its own warnings"
status: todo
priority: 2
assignee:
labels: [missouri, tests]
depends_on: []
created: "2026-08-13T00:55:10Z"
updated: "2026-08-13T00:55:10Z"
---

The nix shell invocation prints 'warning: --no-registries is deprecated' into stderr. Suites that assert exact stderr (zettel: 6 assertions) fail on missouri's own wrapper output, not on the tool under test. missouri must keep its sandbox machinery out of the streams it compares: filter wrapper-origin lines before comparison, or update the deprecated flag, or both. Workaround verified in zettel: MISSOURI_SANDBOX=preinstalled makes the suite green.
