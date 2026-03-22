---
title: "docs subcommand: each tool prints its own bundled docs"
status: done
priority: 2
assignee:
labels: [clc, docs]
depends_on: []
created: 2026-03-21T18:23:34Z
updated: "2026-03-22T02:17:27Z"
---

Each tool gets a docs subcommand that prints its own bundled documentation.

## Commands per tool

tisket docs — list tisket docs
tisket docs <topic> — print a tisket doc
missouri docs — list missouri docs  
missouri docs <topic> — print a missouri doc
clc docs — list all docs (clc's own + ecosystem-level)
clc docs <topic> — print a doc

All three: docs search <query> — search across that tool's docs

## Also via clc delegation

clc tisket docs → delegates to tisket docs
clc missouri docs → delegates to missouri docs

## Implementation

Each crate bundles its own docs/ via include_str!() in a docs module.
Share the docs module structure across crates (maybe in clc-sdk or as
a pattern each crate implements).

- tisket bundles: tisket/docs/*.md
- missouri bundles: missouri/docs/*.md  
- clc bundles: clc/docs/*.md (includes ecosystem docs: index, what-is,
  getting-started, phase-system, orchestration, cli-reference)

## Output format

Raw markdown to stdout. No rendering, no colors. Agents consume directly.
Humans pipe to bat/less/glow.

## Done when

- tisket docs, missouri docs, clc docs each list their docs with titles
- tisket docs <topic> prints the correct doc
- missouri docs <topic> prints the correct doc
- clc docs <topic> prints any doc (clc's own + ecosystem)
- clc tisket docs delegates correctly
- clc missouri docs delegates correctly
- docs search <query> works on each tool
- cargo test covers the commands
