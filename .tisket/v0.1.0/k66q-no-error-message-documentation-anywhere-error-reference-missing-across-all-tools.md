---
title: "no error message documentation anywhere — error reference missing across all tools"
status: todo
priority:
assignee:
labels: [docs, completeness]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

CLI reference docs should document error conditions, exit codes, and error message formats so users and agents can handle failures programmatically. Only missouri's cli-reference.md documents exit codes (0/1/2/130 on line 45). The clc cli-reference.md has zero mentions of errors or exit codes. The tisket cli-reference.md mentions "error" only twice — once for an unimplemented command and once for ambiguous prefix matches — with no systematic error reference. No tool documents its error message format or common failure modes.

An agent encountering an error from clc or tisket has no documentation to consult for what the exit code means, whether the error is recoverable, or what corrective action to take.

## Open Questions

- What are the actual exit code conventions for clc and tisket? Are they consistent with missouri's (0=success, 1=failure, 2=config error)?
- Should error documentation be per-command or a consolidated error reference section?
- Are error messages structured (parseable prefix like `error:`) or freeform?

## Why It Matters

Agents wrap these tools in scripts and automation. Without documented exit codes and error conditions, error handling is guesswork — retry on everything, ignore everything, or hardcode assumptions that break when messages change.
