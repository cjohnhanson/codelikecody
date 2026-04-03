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
