---
title: "Prime text does not include operational instructions — workers don't know how to run tools"
status: done
priority:
assignee:
labels: [clc]
depends_on: []
created: 2026-02-28T19:51:08Z
updated: "2026-02-28T20:12:48Z"
---

## Problem

Workers don't know how to invoke project tooling correctly. Observed: a worker tried running bare `missouri run` instead of `clc missouri run`, getting PATH errors. The prime text already has a missouri section (injected by missouri::detect), but it describes test state ("1 paths, 10 states") without saying HOW to run the tests.

This is a content problem in the prime text, not an architecture problem. Context compaction makes it worse (operational details learned mid-session get dropped), but the root issue is that the prime text never teaches it in the first place.

## What the prime text says now

The missouri section (from missouri's prime() method) reports:
- Number of test paths and states
- Current pass/fail status

It does NOT say:
- How to run the tests (`clc missouri run`)
- Where tests live (`clc/tests/missouri/`)
- What missouri tests are (state-graph e2e tests)

## What it should say

The prime text should include operational instructions: how to invoke each tool the worker needs. If the prime says "you have missouri tests" it should also say how to run them. Same principle for any other tooling.

## Discovery needed

- Review all prime text sections (missouri, tisket, clc header) for similar gaps
- Check whether the tisket prime section tells workers how to use tisket commands
- Check whether the workflow section gives enough operational detail or just describes phases abstractly
