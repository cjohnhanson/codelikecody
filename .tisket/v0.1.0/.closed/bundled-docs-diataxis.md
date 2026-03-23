---
title: "Bundled docs (diataxis)"
status: done
priority:
assignee:
labels: [architecture]
depends_on: [clc-sdk-crate-with-agent-detection]
created: 2026-02-24T14:52:06Z
updated: "2026-03-23T00:21:00Z"
---

Bundle documentation in the binary via `include_str!()` of markdown files,
organized by the Diataxis framework. The binary is the single source of truth
for its own version.

## Diataxis quadrants

- **Tutorials**: Learning-oriented. "Getting started with tisket."
- **How-to guides**: Task-oriented. "How to write a missouri test."
- **Reference**: Information-oriented. Technical details beyond --help.
- **Explanation**: Understanding-oriented. "Why the phase system exists."

## Access

`clc docs`, `clc tisket docs`, `clc missouri docs` — each tool surfaces its own
bundled docs. Agent detection controls output format (plain markdown for agents,
rendered for terminals).

## Implementation

1. Create `docs/` directory in each crate with markdown files per quadrant
2. `include_str!()` in lib.rs to bundle at compile time
3. Add `docs` subcommand to each tool's CLI
4. clc mounts tool docs under `clc <tool> docs`
5. Optionally support `clc docs --topic <name>` for cross-cutting topics

## Note

Doc content (like prime content) is text that enters agent context. Requires
explicit user approval before writing to file.
