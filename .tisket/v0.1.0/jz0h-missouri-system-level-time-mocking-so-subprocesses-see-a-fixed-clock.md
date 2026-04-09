---
title: "missouri: system-level time mocking so subprocesses see a fixed clock"
status: todo
priority:
assignee:
labels: [missouri, feature]
depends_on: []
created: 2026-04-09T03:42:46Z
updated: "2026-04-09T03:43:12Z"
---

## Problem

Missouri tests filesystem state as a graph: each node is a directory snapshot, each edge is a shell command. Time-sensitive code can't be tested this way because:

- Subprocess children see the host's real clock, not a fixed test clock
- Any state comparison involving timestamps (file mtimes, log timestamps, DB timestamps, agent created_at) will diverge across runs
- Tests that depend on 'before X' / 'after X' behavior aren't reproducible

## Inspiration: freezegun

Python's `freezegun` patches `datetime.now()` and `time.time()` at the process level. Works well within a Python process. Doesn't cross into subprocess children.

What we need: a way to set a fixed time that's observable by subprocesses spawned from missouri.

## Possible approaches

1. **libfaketime** (LD_PRELOAD on Linux, DYLD_INSERT_LIBRARIES on macOS) — intercepts `clock_gettime`, `time`, `gettimeofday` at the libc level. Works across subprocesses via environment variables. Used by pytest-freezer for integration tests.

2. **Env var convention** — missouri sets `MISSOURI_FAKE_TIME=2026-04-08T00:00:00Z` and every tool that cares about time reads it. Requires all code to be aware.

3. **A wrapper binary** — missouri runs commands under a wrapper that fakes time using seccomp or ptrace. Heavy.

4. **Isolated namespace** — run in a Linux time namespace (CLONE_NEWTIME). Linux-only, requires root or userns.

## Acceptance criteria

- Decide on an approach
- Document how to set a fixed time for a missouri test
- Add at least one missouri test that verifies deterministic behavior across runs (e.g., a tisket with a fixed created_at)
- Work on both Linux and macOS in the dev environment

## Related

Pairs with the tisket for postgres state testing — both are about making deterministic, reproducible tests of stateful systems.
