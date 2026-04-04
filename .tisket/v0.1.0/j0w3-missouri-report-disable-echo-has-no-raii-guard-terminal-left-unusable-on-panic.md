---
title: "missouri report::disable_echo has no RAII guard — terminal left unusable on panic"
status: todo
priority:
assignee:
labels: [missouri, correctness, auto]
depends_on: []
created: 2026-03-23T03:11:53Z
updated: "2026-04-03T18:32:46Z"
---

## Problem

Terminal echo should be restored unconditionally when `ProgressReporter` is done, regardless of how execution ends. The current code in `missouri/src/report.rs` calls `disable_echo()` at line 68 during `ProgressReporter::new()` and stores the original termios in `original_termios`. Restoration happens in the `Drop` impl (line 147), which calls `restore_termios`. However, if a panic unwinds through a scope that holds `ProgressReporter` and the panic handler aborts or the drop is skipped (e.g., `mem::forget`, double panic), the terminal is left with echo disabled — the user's keystrokes become invisible, making the terminal unusable without `reset` or `stty echo`.

## Open Questions

- Does `Drop` reliably run on panic in missouri's usage, or are there `catch_unwind` boundaries or abort-on-panic configurations that would skip it?
- Should a `ctrlc` or signal handler also restore termios, since SIGKILL and SIGTERM won't run destructors?
- Would wrapping the termios state in a dedicated RAII guard (separate from `ProgressReporter`) make the restoration path clearer and harder to accidentally bypass?

## Why It Matters

A terminal left with echo disabled is effectively broken for interactive use. The user has to know to run `stty echo` or `reset` to recover. This is especially problematic when missouri is run by agents — a crash leaves the agent's terminal in a bad state with no recovery path.

## Scratch Notes

### 2026-04-04: Investigation

This issue is stale. The underlying problem was already fixed by commit `1a9e02b`
("feat: replace indicatif spinners with line-oriented reporting"), which removed
all terminal echo suppression, cursor hiding, and indicatif/termimad/console
dependencies from missouri.

The current `ProgressReporter` in `missouri/src/report.rs` is pure line-oriented
output — no `disable_echo()`, no termios, no terminal state manipulation at all.
Comment on line 8: "No terminal manipulation, no cursor hiding, no echo suppression."

Related closed tisket: `e0rz-missouri-replace-indicatif-progress-spinners-with-line-oriented-reporting-and-timing-summary` (status: done).

Grep for `disable_echo|termios|stty|echo_off|ECHO` across `missouri/` returns zero matches.

**Conclusion**: No tests to write, no code to change. This should be closed as already resolved.
