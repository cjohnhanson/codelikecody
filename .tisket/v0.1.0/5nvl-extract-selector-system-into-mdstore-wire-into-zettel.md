---
title: "Extract selector system into mdstore, wire into zettel"
status: in_progress
priority:
assignee:
labels: [refactor, zettel]
depends_on: []
created: 2026-03-22T12:39:24Z
updated: "2026-03-22T12:39:34Z"
---

Move the generic selector parsing (namespace:value, AND semantics) into mdstore. Tisket and zettel both implement domain-specific matching via a trait.

## What moves to mdstore

- Selector struct and parse method
- matches_all function
- A trait for domain-specific matching

## What stays in each tool

- tisket: matches against Issue (label, status, project, tags fallthrough)
- zettel: matches against Note (tag, status, link)

## Also

- Add --where flag to zettel note list (same as tisket issue list)
- Missouri tests for zettel selectors

## Scratch Notes
