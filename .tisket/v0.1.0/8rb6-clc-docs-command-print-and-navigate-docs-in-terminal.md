---
title: "clc docs command: print and navigate docs in terminal"
status: todo
priority: 2
assignee:
labels: [clc, docs]
depends_on: []
created: 2026-03-21T18:23:34Z
updated: "2026-03-21T18:24:02Z"
---

CLI command for reading project documentation in the terminal.

clc docs — list available docs
clc docs <topic> — print a doc to stdout
clc docs search <query> — search across docs

Markdown from docs/ baked into the binary at compile time. Rendered as
terminal-formatted text (or raw markdown, agent-friendly). No server,
no browser — just print.

Agents use this to look up CLI reference, config schemas, and workflows
without needing web access or file reads. Humans pipe to a pager.
