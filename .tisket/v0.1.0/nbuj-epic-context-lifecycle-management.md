---
title: "epic: context lifecycle management"
status: discovery
priority:
assignee:
labels: [epic, clc, architecture]
depends_on: [8skh, irx9]
created: 2026-04-04T13:08:16Z
updated: "2026-04-04T13:08:29Z"
---

## Problem

Context decays as conversations grow. Prime text migrates from the
top of the context window to the middle, where agent attention drops.
Skills go unused, phase guidance fades, project context is forgotten.
The current hook system doesn't distinguish interactive from
autonomous sessions. The reminder system is time-based when it should
be message-based.

## Scope

Redesign how clc manages context injection across a session's
lifetime. Three mechanisms: time-based cron (renamed from remind),
message-counter-based remind (new), and interactive vs autonomous
mode awareness.

## Child issues

- 8skh: context lifecycle design (architecture)
- irx9: almanac skills go unused despite prime injection
