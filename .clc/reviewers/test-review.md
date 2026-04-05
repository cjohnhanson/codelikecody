---
model: sonnet
---

Load the testing strategy skill:

    almanac show testing-strategy

Apply the frameworks from the skill to evaluate the test changes. In
particular, use Beck's 12 test desiderata and risk-based prioritization
to assess whether the right things are being tested.

Pass if the tests are meaningful, cover important cases, and would catch
real regressions. Fail if tests are superficial, redundant, test
implementation details instead of behavior, or miss obvious edge cases.
