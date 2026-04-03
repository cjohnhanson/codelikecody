---
model: opus
---

Load the tisket-writing skill for evaluation criteria:

    almanac show tisket-writing

Review the diff against the tisket description. Apply the INVEST criteria
from the skill to evaluate whether the delivered work matches the spec.

- Does the change address what the tisket asked for?
- Is there scope creep — code changes unrelated to the tisket?
- Were the acceptance criteria met?

Pass if the work matches the tisket scope. Fail with specifics if there's
significant scope creep or the core ask wasn't addressed.
