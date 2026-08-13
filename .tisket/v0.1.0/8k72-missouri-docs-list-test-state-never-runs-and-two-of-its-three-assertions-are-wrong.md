---
title: "missouri: docs-list test state never runs, and two of its three assertions are wrong"
status: todo
priority:
assignee:
labels: [missouri, testing]
depends_on: []
created: 2026-08-12T21:23:57Z
updated: "2026-08-12T21:24:21Z"
---

## Problem

The `docs-list/has-docs` state in missouri's own test suite has three
assertions that guard the bundled documentation. None of them ever run,
and two of them assert something the code does not do.

Found while converting missouri's docs to controlled English. These
assertions were the stated safety net for doc edits, and the safety net is
not connected.

### Defect 1: the state is never enumerated

`missouri/tests/missouri/docs-list/has-docs/` is a root state with
assertions and no transitions. Path enumeration in
`missouri/src/paths.rs` only emits a path that has at least one
transition, so no path ever includes this state.

Reproduce from `missouri/tests/missouri/`:

    missouri list --show states
      has-docs
      empty
      with-direct-output
      with-shell-output

    missouri run --check-only
      2 passed, 0 failed

`has-docs` is discovered but contributes nothing. `--check-only` runs the
assertions for two states, not three.

### Defect 2: two assertions grep for text the command never prints

`missouri docs` and `missouri docs search` both print slug and
description through `format_list` / `format_list_from_refs` in
`missouri/src/docs.rs`. Neither prints the page title. The assertions grep
for titles.

Actual output of `missouri docs`:

    what-is-missouri   Why filesystem state graphs and how missouri's testing model works
    getting-started    Create your first state graph test suite
    writing-tests      How to model tests as state graphs with transitions, assertions, and services
    cli-reference      Complete command reference for the missouri test framework

Run by hand against a current build:

    missouri docs | grep -q 'What is Missouri' && missouri docs | grep -q 'CLI Reference'   -> FAIL
    missouri docs what-is-missouri | grep -q 'filesystem state'                             -> PASS
    missouri docs search transition | grep -q 'Writing Tests'                               -> FAIL

Only the second assertion is correct. It greps page content, which
`missouri docs <slug>` does print.

## Why it matters

Two separate holes stack here. Assertion-only root states run nothing, so
any suite written that way is silently dead. And the docs assertions
encode a false expectation, so fixing the first defect alone turns the
suite red rather than green.

The wider risk is the pattern: a contributor can add an assertion-only
root state, watch `missouri run` report all green, and believe the state
is covered.

## Acceptance criteria

1. An assertion-only root state runs its assertions. `missouri run` and
   `missouri run --check-only` both report them.
2. `missouri run` from `missouri/tests/missouri/` reports the `has-docs`
   assertions in its output.
3. The three `has-docs` assertions pass. Either grep for text that the
   commands print, or make the commands print the title.
4. A regression test covers an assertion-only root state, so this cannot
   go dead again unnoticed.

## Notes

Decide deliberately on point 3. Either the assertions are wrong, or
`missouri docs` should show titles as well as slugs. Showing the title is
arguably the better CLI, since the slug is what you type and the title is
what you read. That is a product call, not a test fix.

Files:
- missouri/src/paths.rs (path enumeration)
- missouri/src/docs.rs (format_list, format_list_from_refs)
- missouri/tests/missouri/docs-list/has-docs/.missouri/missouri.yml
