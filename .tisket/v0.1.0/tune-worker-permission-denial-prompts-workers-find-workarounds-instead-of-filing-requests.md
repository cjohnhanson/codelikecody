---
title: "tune-worker-permission-denial-prompts-workers-find-workarounds-instead-of-filing-requests"
status: todo
priority:
assignee:
labels: []
depends_on: []
created: "2026-03-01T23:26:36Z"
updated: "2026-03-01T23:26:36Z"
---

When a worker hits a permission denial in dontAsk mode, Claude's built-in denial
message says "You *may* attempt to accomplish this action using other tools." Workers
take this literally and route around restrictions (e.g., letting Missouri's test
sandbox run the denied command instead of calling it directly).

The worker system prompt says to run `clc permissions request` and stop, but the
built-in denial message contradicts this by encouraging workarounds.

Investigate:
- Whether the denial message can be customized (canUseTool callback not available
  in --print mode, but check if there's a settings-level override)
- Whether the worker system prompt needs stronger language ("do not attempt
  workarounds — file a request immediately")
- Whether the coordinator prompt needs to detect and correct workaround behavior
- Whether dontAsk mode's denial message is configurable in Claude Code settings

Observed behavior: worker dispatched on a task requiring `make --version`, got denied
on Bash(make *), then let Missouri's test runner execute make through its sandbox
transitions instead of filing a permission request. Task completed without the
permission system ever being engaged.
