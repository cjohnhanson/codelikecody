---
title: "missouri: replace indicatif progress spinners with line-oriented reporting and timing summary"
status: in_progress
priority: 2
assignee:
labels: [missouri, reporting]
depends_on: []
created: 2026-03-27T01:49:23Z
updated: "2026-03-27T01:55:22Z"
---

## Problem

Missouri's test output uses `indicatif` MultiProgress spinners with
terminal echo suppression and cursor hiding. This causes:

1. Terminal left in broken state when process is killed mid-run (echo
   disabled, cursor hidden). The `disable_echo()` has no signal handler
   or RAII guard — only `Drop` on `ProgressReporter`, which doesn't run
   on SIGKILL.
2. Frozen scrollback in terminal multiplexers — `indicatif` redraws in
   place, fighting with tmux/zellij.
3. Output isn't pipeable or greppable — the redraws produce garbage in
   log files.
4. No per-assertion timing — can't tell which assertions are slow.
5. No timing summary — no slowest transitions, no wall-time vs CPU-time
   breakdown, no optimization signal.

The reporting also doesn't show per-assertion timing, doesn't rank slow
transitions, and doesn't distinguish wall time from sum-of-path time
(paths run in parallel via rayon).

## Acceptance Criteria

- [ ] Remove `indicatif` dependency and all terminal manipulation
      (disable_echo, hide_cursor, MultiProgress)
- [ ] Replace with line-oriented output: one line per event, printed
      as it happens, pipe-friendly
- [ ] Per-path timing: `PASS initialized → has-config 1.2s`
- [ ] Per-transition timing in verbose mode: `  ✓ init → configured (run config) 450ms`
- [ ] Per-assertion timing in verbose mode: `    ✓ config file exists 12ms`
- [ ] Progress fraction: `[12/38] initialized → has-config`
- [ ] Summary includes: wall time, total paths, steps, assertions
- [ ] Summary includes top 5 slowest transitions with timing
- [ ] Failed assertion output includes: the command, exit code,
      stdout/stderr (unchanged from current behavior)
- [ ] No terminal state corruption on kill — no echo suppression,
      no cursor manipulation

## Out of Scope

- JSON output mode (useful but separate concern)
- Color themes or configuration
- Test filtering or selective re-run

## Done When

- `missouri run` produces clean, greppable output
- `missouri run -v` shows per-transition and per-assertion timing
- Summary shows slowest transitions
- Killing missouri mid-run leaves terminal in normal state
- All existing missouri test suites still pass

## Scratch Notes
