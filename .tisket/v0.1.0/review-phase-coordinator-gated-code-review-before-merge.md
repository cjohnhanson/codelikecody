---
title: "Review phase: coordinator-gated code review before merge"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-03-01T13:04:59Z
updated: "2026-03-03T04:44:35Z"
---

Add a review phase to the worker lifecycle so the coordinator gates what lands.

## Flow

1. Worker reaches green, advances to `review-requested`, stops
2. Coordinator sees `review-requested`, dispatches review worker in the same worktree, phase becomes `in-review`
3. Review worker reads diff, runs tests, checks quality, writes structured report to `.clc/review/`, advances to `reviewed`, stops
4. Coordinator sees `reviewed`, reads `.clc/review/`, decides:
   - Accept: advances to `done`, calls `clc done`, merges
   - Reject: summarizes blockers into a focused dispatch prompt, kicks back to `implementing`, dispatches fix worker with that prompt
5. Fix worker addresses only the blockers, reaches green → back to step 1
6. After 3 review cycles with no acceptance, coordinator escalates to the user instead of looping

## Review worker checklist

The review worker fills out a structured checklist and writes findings:

```json
{
  "findings": [
    {
      "category": "tests",
      "severity": "blocker",
      "description": "No missouri test for the new state transition",
      "file": "clc/src/phase.rs",
      "line": 42
    }
  ],
  "checklist": {
    "matches_tisket": true,
    "has_tests": true,
    "has_missouri_coverage": false,
    "follows_patterns": true,
    "no_duplicate_patterns": true,
    "no_rule_violations": true
  }
}
```

### Checklist items

- **matches_tisket** — code does what the tisket asked for
- **has_tests** — unit tests exist for new behavior
- **has_missouri_coverage** — state-graph tests where applicable (not every tisket needs them)
- **follows_patterns** — matches existing conventions in the codebase
- **no_duplicate_patterns** — doesn't introduce a new module/class/pattern that duplicates something that already exists
- **no_rule_violations** — no shelling out to git, no hardcoded paths, etc.

## Coordinator decision logic

- Any `blocker` severity finding → reject
- All checklist items true and no blockers → accept
- Warnings are forwarded to the worker but don't block acceptance
- No subjective judgment — either it meets the bar or it doesn't

## Rejection flow

When the coordinator rejects:
- `.clc/review/` artifacts accumulate (timestamped) for auditability
- The fix worker does NOT read `.clc/review/` directly
- The coordinator distills blockers into a focused instruction and dispatches the fix worker with that as the prompt
- This prevents the fix worker from over-correcting — it addresses only the specific blockers

## Review worker context

The review worker prompt includes:
- The tisket (what was asked for)
- The diff (`git diff main...HEAD`)
- The project's rule list (no shelling out, no hardcoded paths, etc.)
- What Missouri is and how to check coverage
- The checklist it needs to fill out
- Explicit instruction that it produces a report, not a verdict

## Key constraints

- Review worker produces a report, never a verdict. Coordinator is the only merge authority.
- Review worker runs in the same worktree as the implementation worker.
- `.clc/review/` artifacts accumulate across review cycles (timestamped or indexed), not overwrite.
- Maximum 3 review cycles before escalating to the user.

## New phases

- `review-requested` (between green and done, Stop hook allows exit)
- `in-review` (coordinator sets when dispatching reviewer)
- `reviewed` (reviewer sets after writing findings)

## Changes needed

- phase.rs: add three new phases
- guard.rs: Stop hook allows exit at `review-requested` and `reviewed`
- coordinate.rs: detect `review-requested`, dispatch reviewer, read `reviewed` artifacts, apply decision logic, handle rejection dispatch with summarized blockers, enforce 3-cycle limit
- done.rs: only callable from `done` phase (coordinator advances to done before calling)
- New: review worker prime text / prompt (requires user approval per CLAUDE.md)

## Scratch Notes

### Tests written (phase: tests-unwritten → tests-written)

**Unit tests:**
- phase.rs: 15 tests — parse, display, ordering, next(), transitions for ReviewRequested/InReview/Reviewed
- guard.rs: 10 tests — Stop allows review-requested/reviewed, blocks in-review; PreToolUse: in-review unrestricted, review-requested/reviewed restricted

**Missouri states:**
- phase-review-requested: assertions + transitions (from phase-green, forward to phase-in-review, back to implementing)
- phase-in-review: assertions + transitions (unrestricted like implementing, Stop blocks, forward to reviewed)
- phase-reviewed: assertions + transitions (Stop allows, forward to done, rejection back to implementing)
- phase-done-via-review: terminal state verifying phase=done after review chain
- ready-to-done-via-review: integration test for `clc done` from done phase (full project setup)
- ready-to-done-via-review-pickup-style: same but with pre-committed tisket

**Key decisions:**
- `in-review` phase is unrestricted (like implementing) — review worker needs write access for `.clc/review/`
- `review-requested` and `reviewed` are restricted (test paths only)
- `clc done` now requires phase=done, not green. Coordinator must advance to done first.
- No done.rs unit tests — they require git repos which would mean shelling out to git. Testing done.rs behavior through missouri instead.
- Updated ready-to-done and ready-to-done-pickup-style: removed direct `clc done` transitions, added review chain transitions
- done-completed's "done fails when already done" assertion still valid — checks tisket already closed

**Files modified:**
- clc/src/phase.rs (unit tests)
- clc/src/guard.rs (unit tests)
- clc/tests/missouri/phase-green/.missouri/missouri.yml (added review-requested transition)
- clc/tests/missouri/ready-to-done/.missouri/missouri.yml (rewrote: removed old done tests, added review chain)
- clc/tests/missouri/ready-to-done-pickup-style/.missouri/missouri.yml (same treatment)

**New files:**
- clc/tests/missouri/phase-review-requested/ (full state)
- clc/tests/missouri/phase-in-review/ (full state)
- clc/tests/missouri/phase-reviewed/ (full state)
- clc/tests/missouri/phase-done-via-review/ (full state)
- clc/tests/missouri/ready-to-done-via-review/ (full state)
- clc/tests/missouri/ready-to-done-via-review-pickup-style/ (full state)

**Next steps:**
- Advance to tests-written
- Implement: phase.rs (add 3 variants), guard.rs (Stop + PreToolUse), done.rs (require Done phase)
- coordinate.rs: review loop (detect review-requested, dispatch reviewer, read reviewed, accept/reject, 3-cycle limit)
- Review worker prompt (needs user approval per CLAUDE.md)
