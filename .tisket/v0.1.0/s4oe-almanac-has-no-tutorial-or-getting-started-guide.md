---
title: "almanac has no tutorial or getting-started guide"
status: discovery
priority:
assignee:
labels: [almanac, docs]
depends_on: []
created: 2026-03-23T03:12:16Z
updated: "2026-03-23T03:53:11Z"
---

## Problem

Every tool in the project should have a getting-started tutorial so new users can go from zero to working. The almanac/docs/ directory contains only two files: cli-reference.md and what-is-almanac.md. There's no getting-started.md. clc, missouri, and tisket all have getting-started pages that walk through install, first use, and verification. Almanac has a CLI reference (commands exist) and an explanation (concepts exist) but no tutorial connecting the two.

## Open Questions

- What's the minimal useful almanac tutorial? List skills, show a skill, configure sources in clc.yml?
- Should the tutorial cover standalone almanac usage, clc-mounted usage, or both?
- Is almanac stable enough for a tutorial, or is the API still in flux?

## Why It Matters

An agent or user encountering almanac for the first time has to piece together usage from the CLI reference flags and the what-is explanation. The gap between "here's what it is" and "here are the commands" is exactly what a tutorial fills.
