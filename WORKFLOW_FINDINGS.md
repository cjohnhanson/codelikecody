# Workflow Analysis: Pure Test-Improvement Tasks

## Summary

The current phase system (tests-unwritten → tests-written → red → implementing → green → done) **does work for pure test-only tasks**, but with noted considerations about ceremony overhead and practical workflow experience.

## Questions Answered

### 1. Does the current phase system work for test-only tasks?

**Answer: Yes, it works, but with caveats.**

The guard system correctly allows test-file edits during the tests-unwritten and tests-written phases because all edits ARE test edits. The phase gates don't create hard blocking for test-only work.

**Observation**: In tests-unwritten and tests-written phases, you can freely edit test files without restriction, which is correct behavior.

### 2. Should there be a separate phase progression for test work?

**Answer: Probably not necessary, but consider a fast-path variant.**

A separate phase progression (like `tests-unwritten → tests-written → done`) would be simpler for pure test tasks, but it creates maintenance burden:
- Two parallel workflows to document and teach
- Testing/validation becomes more complex (which path should be used when?)
- Hybrid tasks (test improvements + small source refactors) don't fit cleanly

**Alternative**: Instead of separate phases, consider a **phase-skipping pattern** that's well-documented: "For test-only work, advance quickly through phases without ceremony."

### 3. Is the existing system fine if you just skip through phases quickly?

**Answer: Yes, mostly. The phrase "ceremony" is apt.**

Working through this task revealed the actual experience:
1. tests-unwritten → write test definitions ✓ (smooth)
2. tests-written → advance phase ✓ (smooth, just a state transition)
3. red → tests fail ✓ (expected, makes sense)
4. implementing → fix tests ✓ (smooth, all edits unlocked)
5. green → tests pass ✓ (verify with `clc status`)
6. done → finalize ✓ (smooth)

The **ceremony is real but lightweight**. Each phase is literally `clc status set <phase>`. The cognitive overhead is minimal when understood.

**Key insight**: For test-only work, phases 3-4 (red → implementing) feel slightly decoupled from the task flow, but they serve a purpose: red confirms tests fail before you try to fix them.

### 4. What about purely structural Missouri test improvements?

**Answer: These flow through the system naturally.**

Adding new Missouri test states/transitions is still test work:
- It's not blocked by the guard (test files)
- It flows through the same phase progression
- The phases provide useful checkpoints even for structural work

## Recommendations

### Keep the Current System For:
- Consistency across all work (implementation and test)
- Clear phase semantics (each phase has a meaningful name)
- Simplicity (one workflow, not multiple)

### Add Documentation For:
- **Fast-path documentation**: How to quickly move through phases for test-only work
- **When each phase matters**: Which phases provide real value for pure test tasks
  - tests-unwritten → tests-written: Essential (marks intent)
  - red: Optional but useful (confirms tests fail before fixing)
  - implementing → green: Necessary (actually do the work)
  - done: Essential (finalizes work)
- **Skipping strategies**: Which phases can be combined/rushed without losing value

### Consider Minor Improvements:
1. **Phase naming for clarity**: The red/implementing split could be explained better
   - "red" = tests written but not yet passing
   - "implementing" = making changes to pass tests
   - This is clear for implementation work but less obvious for test work

2. **Guard refinement**: Current guard is correct but could have clearer messaging
   - Example: "non-test edits blocked; test-only work can proceed"

3. **Fast-track mode consideration**: Optional mode that collapses phases for test-only work
   - NOT recommended: would create fork in workflow
   - INSTEAD: Document the pattern of quick advancement

## Conclusion

**The answer to "does the phase system work for test-only tasks?" is yes.**

- ✓ The guard correctly allows test edits
- ✓ Phases provide useful checkpoints
- ✓ The "ceremony" is minimal and serves a purpose
- ✓ No separate system is needed; fast-pathing through existing phases is reasonable

The existing system is **fit for purpose** and should be maintained as the single source of truth for all work. The remedy for "ceremony overhead" is better documentation of the fast-path pattern for test-only work, not architectural changes.
