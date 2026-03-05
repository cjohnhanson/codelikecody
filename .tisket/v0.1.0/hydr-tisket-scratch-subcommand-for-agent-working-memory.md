---
title: "tisket scratch subcommand for agent working memory"
status: in_progress
priority:
assignee:
labels: [tisket, ergonomics]
depends_on: []
created: 2026-03-05T04:30:56Z
updated: "2026-03-05T04:33:02Z"
---

## Problem

Agent working memory lives in a `## Scratch Notes` section of the tisket file. Currently requires file I/O to read or write — which is blocked on trunk and error-prone everywhere else.

This is the most common agent file operation. It needs a dedicated CLI path.

## Design

Top-level subcommand (not buried under `issue`):

- `tisket scratch <id>` — print the scratch notes section (default: read)
- `tisket scratch <id> read` — same, explicit
- `tisket scratch <id> append "text"` — append text to scratch notes
- `tisket scratch <id> write "text"` — replace the entire scratch section
- `tisket scratch <id> clear` — wipe scratch notes

Auto-creates `## Scratch Notes` heading on first write if the section doesn't exist.

## Implementation

- New `scratch` subcommand in tisket CLI
- Parse markdown body to find `## Scratch Notes` section boundaries
- Read: extract and print just that section
- Write/append/clear: modify just that section, preserve everything else
- Section detection: look for `## Scratch Notes` heading, content runs until next `## ` heading or EOF

## Verification

- Create issue, scratch read returns empty
- Scratch append adds text, scratch read shows it
- Scratch write replaces, scratch append after write appends
- Scratch clear wipes, scratch read returns empty
- Operations work on issues that don't yet have a scratch section
- Existing body content outside scratch section is preserved

## Scratch Notes
