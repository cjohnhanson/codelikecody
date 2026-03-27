---
title: "migrate missouri browser tests from playwright to moose — replace playwright-missouri skill and patterns"
status: discovery
priority: 3
assignee:
labels: [missouri, moose, skills]
depends_on: []
created: "2026-03-27T02:21:40Z"
updated: "2026-03-27T02:21:40Z"
---

## Problem

Missouri browser tests currently use Playwright via the `playwright-missouri`
skill. Moose is now in the workspace and provides the same capabilities —
CDP-based browser automation with accessibility tree snapshots, screenshots,
network interception, and form interaction. Having two browser automation
tools creates confusion about which to use and adds an external dependency
(Node.js + Playwright) that moose eliminates.

The accessibility tree snapshot from `moose snapshot` is deterministic for
a given DOM state — plain text, diffable, no external rendering dependencies.
This maps directly to missouri's filesystem comparison model: save the
snapshot output as a fixture file, missouri diffs the actual output against
it.

## What needs to change

1. **`playwright-missouri` skill** → rewrite as `moose-missouri` or fold
   into the existing `moose` skill. The patterns are the same (snapshot as
   state representation, assertions on page content) but the commands are
   different.

2. **Transition commands** in any missouri test that uses Playwright scripts
   → replace with `moose` CLI commands. Playwright scripts are JS files;
   moose commands are shell one-liners that work directly in missouri
   transition `command:` fields.

3. **Snapshot comparison** — the primary page state representation becomes
   `moose snapshot` output saved as a text fixture. Missouri diffs it like
   any other file. Could use a custom comparator for semantic comparison
   (ignore whitespace, ref IDs that change).

4. **Screenshot comparison** — `moose screenshot` produces PNGs. Missouri
   can use `moose diff screenshot --baseline` as a custom comparator for
   visual regression.

5. **Documentation** — update missouri docs (`writing-tests.md`) to show
   moose patterns instead of Playwright patterns.

## Open Questions

- Should `moose snapshot` output be normalized before comparison? Ref IDs
  (`ref=e1`, `ref=e2`) might change between runs if the DOM order changes
  slightly. A comparator that strips ref IDs would make snapshots more
  stable.
- Should the `playwright-missouri` skill be removed entirely or kept as
  deprecated? Projects outside codelikecody might still use Playwright.
- How to handle Playwright's `page.evaluate()` patterns — moose has
  `moose eval` but the invocation is different.

## Why It Matters

Moose is in the workspace, built from source, no Node.js dependency. Using
it for missouri browser tests means the entire test infrastructure is Rust
all the way down — missouri orchestrates, moose automates the browser,
no external runtime needed.
