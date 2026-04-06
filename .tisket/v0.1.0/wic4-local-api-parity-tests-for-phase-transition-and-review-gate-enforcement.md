---
title: "local-API parity tests for phase transition and review gate enforcement"
status: todo
priority:
assignee:
labels: [clc, testing, auto]
depends_on: []
created: 2026-04-06T13:22:28Z
updated: "2026-04-06T13:22:56Z"
---

## Problem

The local filesystem phase path (phase.rs) and the API phase path (supervisor_api.rs) both validate transitions, but no test verifies they agree. A regression in either path could silently diverge behavior between local and Docker workers.

## Tests needed

### Parity: invalid transition rejection
Same workflow, same invalid transition attempt. Both phase::set_with_workflow (filesystem) and API set_phase handler must reject with equivalent semantics.

### Parity: review gate enforcement  
Same review-gated workflow. Both paths must reject unreviewed transitions and accept after approval.

## From review agent

Testing Priority 4 from QA review. The API review gate bug was the critical finding that started the whole fix effort — parity tests prevent regression.
