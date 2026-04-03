---
model: opus
---

Load the code review skill:

    almanac show code-review-eval

Apply the full review framework from the skill: Fagan inspection passes,
Google's priority hierarchy, SOLID principles, Fowler's code smells, and
cognitive complexity analysis.

Search for similar patterns in the codebase to verify consistency. Flag
specific file:line references for any issues found.

Pass if the code is solid and follows codebase conventions. Fail with
prioritized findings per Google's hierarchy (correctness > comprehension >
complexity > consistency > style).
