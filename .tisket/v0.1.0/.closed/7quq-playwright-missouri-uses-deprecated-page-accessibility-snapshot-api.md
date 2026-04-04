---
title: "playwright-missouri uses deprecated page.accessibility.snapshot API"
status: done
priority:
assignee:
labels: [skills, accuracy, standard]
depends_on: []
created: 2026-03-23T03:12:25Z
updated: "2026-04-04T13:39:58Z"
---

## Problem

1. Skill code examples should use current, supported APIs so that agents copying the examples produce working code.
2. In `skills/playwright-missouri/SKILL.md`, line 78 of the example Playwright script uses `page.accessibility.snapshot()` to capture the accessibility tree. This API was deprecated in Playwright 1.41 (December 2023) and has been replaced by the `aria_snapshot` assertion pattern and `locator.aria_snapshot()`. The deprecated API may be removed in a future Playwright release.
3. An agent following this skill will produce scripts that emit deprecation warnings today and will break entirely when the API is removed. The skill is also the canonical teaching document for how to capture accessibility state in missouri browser tests — if the example is wrong, every test written from it will be wrong.

## Open Questions

- Should the replacement use `page.locator('body').aria_snapshot()` for a full-page snapshot, or switch to targeted `expect(locator).to_match_aria_snapshot()` assertions?
- Does the YAML output format of the old `accessibility.snapshot()` differ from the new `aria_snapshot()` format? If so, the comparator guidance ("ARIA snapshots as YAML are directly text-diffable") may also need updating.
- Should the skill pin a minimum Playwright version in the PEP 723 dependencies to ensure the replacement API is available?

## Why It Matters

The skill's primary code example uses a deprecated API. Every browser test written following this skill inherits the deprecation, and the breakage will be silent until Playwright removes the API entirely.
