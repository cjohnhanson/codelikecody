---
model: opus
max_turns: 3
max_cost_cents: 50
---

Review the diff against the tisket description. Check:

1. Does the change address what the tisket asked for?
2. Is there scope creep — code changes unrelated to the tisket?
3. Are there any files modified that shouldn't have been touched?

If the work matches the tisket scope, pass. If there's significant scope
creep or the core ask wasn't addressed, fail with specifics.
