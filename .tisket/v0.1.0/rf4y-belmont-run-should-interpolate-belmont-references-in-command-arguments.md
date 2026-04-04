---
title: "belmont run should interpolate belmont:// references in command arguments"
status: discovery
priority:
assignee:
labels: [enhancement, belmont]
depends_on: []
created: "2026-04-04T14:14:35Z"
updated: "2026-04-04T14:14:35Z"
---

## Problem

belmont run sets secrets as environment variables on the child process
and scrubs them from output. But there's no input-side processing.
An agent that writes belmont://SECRET_NAME in a command argument gets
the literal string, not the resolved value.

The intended flow would be:
1. Agent writes: belmont run -- echo belmont://SECRET_NAME
2. Belmont resolves SECRET_NAME from backend
3. Belmont replaces belmont://SECRET_NAME with the actual value in
   the command arguments before executing
4. Command runs with the real value
5. Output scrubber replaces the real value back to belmont://SECRET_NAME

This would let agents reference secrets in command arguments without
the shell expanding environment variables (which is visible in process
listings and /proc).

## Open Questions

- Is this the right syntax? belmont:// as the interpolation marker
  matches the scrubber's replacement format, which is nice
- Should interpolation happen in all arguments or only specific flags?
- What about nested interpolation (belmont:// inside a quoted string)?
- Security: does pre-processing command arguments introduce new
  exfiltration vectors?
