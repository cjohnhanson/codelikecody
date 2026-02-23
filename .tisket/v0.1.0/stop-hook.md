---
title: "Stop hook"
status: discovery
priority:
assignee:
labels: [feature]
depends_on: [clc-init, status-transitions, event-system-and-agent-adapter]
created: "2026-02-23T02:23:25Z"
updated: "2026-02-23T02:23:25Z"
---

Stop hook blocks the agent from stopping until `clc done` has been called. If
the agent tries to stop and the phase isn't "done", the hook blocks with a
message about completing the work first.

## Missouri tests

Assertions (pipe Stop event JSON to `clc hook`):
- Phase=implementing: Stop blocked (exit 2), message says to run `clc done`
- Phase=done: Stop allowed (exit 0, passthrough)
- On main (no active work): Stop allowed
