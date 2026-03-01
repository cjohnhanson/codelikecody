---
title: "Prime text should mandate TDD independently of phase enforcement"
status: in_progress
priority: 2
assignee:
labels: [clc, prompt]
depends_on: []
created: 2026-02-26T04:13:11Z
updated: "2026-03-01T16:51:49Z"
---

The prime text describes TDD as part of the workflow system — "Write tests first —
phase gates prevent implementation until tests exist." This documents what the hooks
do, it doesn't independently direct the agent. When phase enforcement isn't active,
the prompt doesn't compensate.

The phase-adapted directives ("This issue defines the work. Review the requirements
above before implementing.") only fire when a phase is set. No phase, no directive.

The prime text needs a standing TDD mandate that fires regardless of phase state.
The hooks are the guardrail; the prompt is the training. Both should work
independently — defense in depth for agent behavior.
