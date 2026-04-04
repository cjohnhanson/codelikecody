---
title: "default moose to lightpanda engine instead of chrome"
status: todo
priority:
assignee:
labels: [enhancement, moose]
depends_on: []
created: 2026-04-04T12:31:00Z
updated: "2026-04-04T12:53:22Z"
---

## Problem

Moose defaults to Chrome as the browser engine (`--engine` flag,
`MOOSE_ENGINE` env var). For headless agent use cases, Chrome is
heavyweight: large binary, high memory usage, complex process
lifecycle. Lightpanda is already supported as an alternative engine
and is purpose-built for headless programmatic browsing.

Agents running moose in Docker workers or CI don't need Chrome's
rendering fidelity. They need fast page loads and DOM access.
Defaulting to Chrome means every workspace needs Chrome installed
even when Lightpanda would suffice.

## Acceptance Criteria

- [ ] Given Lightpanda is on PATH, when moose is invoked without
      `--engine`, then Lightpanda is used
- [ ] Given Lightpanda is not on PATH, when moose is invoked
      without `--engine`, then Chrome is used as fallback
- [ ] Given `--engine chrome` is explicitly passed, then Chrome is
      used regardless of Lightpanda availability
- [ ] Given `MOOSE_ENGINE=chrome` is set, then Chrome is used
      regardless of Lightpanda availability

## Out of Scope

- Bundling or installing Lightpanda automatically
- Removing Chrome support

## Done When

- Default engine selection prefers Lightpanda when available
- `--engine` flag and `MOOSE_ENGINE` env var still override the default
- `moose --help` documents the changed default behavior
- Existing tests pass
