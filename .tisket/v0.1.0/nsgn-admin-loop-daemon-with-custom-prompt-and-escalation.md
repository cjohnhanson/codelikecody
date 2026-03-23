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

## Recent Changes (io9i)

The io9i landing built out two layers that sit below the admin loop:

- **coordinator_loop.rs** handles mid-chain decisions: permission grants, worker dispatching, status monitoring, and auto-restart of failed workers. This is the coordination tier that the admin loop would delegate to.
- **supervisor.rs** handles process-level monitoring and restart — keeping coordinator processes alive and surfacing escalations.

What's still missing is the admin loop itself: an LLM-powered daemon with its own custom prompt (referenced from clc.yaml), inbox/outbox polling, and the admin-tier escalation logic. The coordinator and supervisor layers handle the mechanical parts (process health, worker lifecycle), but nothing yet makes LLM-driven decisions about what to do with escalated issues, when to create tiskets, or when to page a human.

Depends on: inbox trait, outbox trait, clc.yaml schema, workspace wiring
Blocks: clc up
