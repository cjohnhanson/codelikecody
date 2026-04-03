---
model: opus
---

Load the design and performance evaluation skills:

    almanac show design-review
    almanac show performance-eval

Apply Nielsen's heuristics and Gestalt principles from design-review to
evaluate UI changes. Apply RAIL model and Core Web Vitals awareness from
performance-eval for any rendering or interaction changes.

Check that:
- UI changes follow established design patterns in the project
- Interaction patterns are consistent and predictable
- No obvious accessibility regressions
- No performance anti-patterns introduced

Pass if the UI work is consistent and usable. Fail with specific
heuristic violations from the skill frameworks.
