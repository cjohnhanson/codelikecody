---
model: opus
max_turns: 5
max_cost_cents: 100
---

Review the code changes for quality. Check:

1. Does it follow existing patterns in the codebase? Search for similar
   code and verify the new code is consistent.
2. Is there unnecessary abstraction — helpers for one-time operations,
   premature generalization, speculative interfaces?
3. Are module boundaries respected — does this code reach into internals
   it shouldn't?
4. Error handling: is it appropriate for the context? No swallowed errors,
   no panic where Result would do, no excessive defensive coding.
5. Is the code clear without comments? If comments are needed, do they
   explain why, not what?

Pass if the code is solid, follows conventions, and doesn't introduce
structural problems. Fail with specific file:line references.
