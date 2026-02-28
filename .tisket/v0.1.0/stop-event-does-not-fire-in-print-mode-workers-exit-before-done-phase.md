---
title: "Workers stop at green phase because check_stop allows green as a valid stop point"
status: in_progress
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T19:51:07Z
updated: "2026-02-28T20:06:26Z"
---

## Problem

Workers stop at green phase and never run `clc done`. The Stop hook fires, but `check_stop()` in guard.rs line 72 returns `Passthrough` for green:

    Some(Phase::Done | Phase::Green) => return Response::Passthrough,

This was correct for interactive use (human runs `clc done` themselves) but wrong for autonomous workers who should run `clc done` before stopping.

## What actually happens

1. Worker reaches green (all tests pass, code committed)
2. Model decides to stop, says "ready for clc done whenever you want"
3. Stop hook fires, calls check_stop()
4. check_stop() sees phase == Green, returns Passthrough (allowed)
5. Worker exits without running `clc done`

## Previous incorrect hypothesis

Earlier investigation grepped stdout.jsonl for "Stop" and found 0 matches, concluding the Stop event never fires in --print mode. This was wrong -- the NDJSON output stream doesn't necessarily contain hook event names. Claude Code docs say only PermissionRequest hooks are explicitly disabled in non-interactive mode. The Stop hook IS registered in the worker settings.local.json and does fire.

## Two distinct failure patterns

1. **Stop at green** (failure modes #5) -- check_stop() allows it. Fix: remove Green from the passthrough match in check_stop().
2. **Stop at implementing** (failure mode #2) -- check_stop() should block this. Needs separate investigation. Either the hook errored, the model found a way around it, or something else. May need logging in the hook to confirm.

## Fix for green-phase stops

Remove Green from the passthrough arm:

    Some(Phase::Done) => return Response::Passthrough,

This means workers (and interactive sessions) must reach done before stopping. The prime text already tells agents to run `clc done` at green -- this makes the stop hook enforce it mechanically.

## Open question: implementing-phase stops

Failure mode #2 (worker stopped at implementing) should already be blocked by check_stop(). Why did the worker exit? Possible explanations:
- The hook errored and Claude Code treated errors as passthrough
- The model exited via a different mechanism than the Stop event
- Something else

Adding logging to the Stop hook would help diagnose. Until then, auto-resume as a safety net (detect premature exit, resume worker) is the pragmatic mitigation.
