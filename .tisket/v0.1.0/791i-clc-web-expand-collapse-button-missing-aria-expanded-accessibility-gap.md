---
title: "clc-web expand/collapse button missing aria-expanded — accessibility gap"
status: todo
priority:
assignee:
labels: [clc-web, accessibility]
depends_on: []
created: "2026-03-23T03:12:16Z"
updated: "2026-03-23T03:12:16Z"
---

## Problem

1. Buttons that toggle visibility of content should communicate their expanded/collapsed state to assistive technology via `aria-expanded`, per WAI-ARIA authoring practices for disclosure widgets.
2. In `clc-web/src/pages/board.rs`, the `StatusColumn` component (lines 96-108) renders a `<button>` that toggles between showing all issues and showing only the first 6. The button text changes ("Show N more" / "Collapse") but there is no `aria-expanded` attribute on the button element. The toggled content region also lacks an `aria-controls` or `id` linking the button to the content it controls.
3. Screen reader users have no programmatic way to determine whether a status column is expanded or collapsed. The button's role is ambiguous — it could be any button doing anything.

## Open Questions

- Should the expandable issue list have an `id` so the button can reference it via `aria-controls`?
- Should the entire expand/collapse pattern be extracted into a reusable `Disclosure` component with accessibility baked in?
- Are there other interactive patterns in clc-web (e.g., future filter panels, detail sections) that will need the same treatment?

## Why It Matters

Missing `aria-expanded` is a WCAG 2.1 Level A failure (4.1.2 Name, Role, Value). It's also a straightforward fix — a single dynamic attribute on an existing element.
