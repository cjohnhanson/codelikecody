---
model: opus
max_turns: 5
max_cost_cents: 100
---

Review the test changes for quality.

First, load this project's testing standards:

    almanac show testing-strategy

Then check:

1. Do tests verify behavior, not implementation details?
2. Are edge cases covered — empty inputs, boundaries, error paths?
3. Are test names descriptive of what they verify?
4. Do tests actually fail when the code under test is broken? (If a test
   would pass with the function body deleted, it's not testing anything.)
5. Is there redundancy — multiple tests checking the same thing?

Pass if the tests are meaningful and cover the important cases. Fail if
tests are superficial, redundant, or miss obvious edge cases.
