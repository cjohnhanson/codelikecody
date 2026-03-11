---
title: "Admin loop daemon with custom prompt and escalation"
status: discovery
priority:
assignee:
labels: [agents]
depends_on: []
created: "2026-03-11T02:20:20Z"
updated: "2026-03-11T02:20:20Z"
---

The admin loop is an async daemon that: polls inboxes for new items, applies decision logic (via LLM with a custom prompt from clc.yaml), takes direct actions (create tiskets, dispatch coordinators), writes to outboxes when human attention is needed.

The admin prompt is a markdown file referenced in clc.yaml. It gives the LLM instructions on how to interpret inbox items and what actions to take. Different repos have different admin prompts.

Escalation chain: worker -> coordinator -> admin -> outbox -> human -> inbox -> admin.

Design question: is the admin loop a plain Rust event loop that calls an LLM API for decisions, or is it an agent session like workers/coordinators? Probably the former — it needs to be long-running and stateful in ways an LLM session isn't.

Depends on: inbox trait, outbox trait, clc.yaml schema, workspace wiring
Blocks: clc up
