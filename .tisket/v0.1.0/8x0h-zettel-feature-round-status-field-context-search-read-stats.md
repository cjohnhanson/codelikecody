---
title: "Zettel feature round: status field, context, search, read, stats"
status: in_progress
priority:
assignee:
labels: [zettel]
depends_on: []
created: 2026-03-22T03:04:06Z
updated: "2026-03-22T03:07:08Z"
---

Add the features needed to make zettel actually useful for knowledge exploration.

## Status field

Add a status field to NoteFrontmatter: draft and permanent. Default to draft on creation. Add --status filter to note list. Agents create drafts, humans promote to permanent.

## Context command

zettel context <id> [-d depth] — show the note plus everything within N hops (default 2). Output full note content (frontmatter + body), not just titles. This is how an agent loads relevant knowledge before starting work.

## Search command

zettel search <pattern> — regex search across note titles, tags, and body text. Same pattern as tisket search. Return matching notes with which fields matched.

## Read command

zettel read --tag <tag> [--status <status>] — dump full content of matching notes. Designed for agent context loading: zettel read --tag auth gives the agent everything known about auth in one shot.

## Stats command

zettel stats — note count, draft vs permanent breakdown, tag distribution, most-connected notes (by backlink count), orphan count. Birds-eye view of knowledge base health.

## Testing

Missouri tests for each feature. Extend the existing test graph with new states.

## Scratch Notes
