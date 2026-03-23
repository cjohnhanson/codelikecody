---
title: "missouri tests use unsafe env var mutation without synchronization under rayon parallelism"
status: todo
priority:
assignee:
labels: [missouri, correctness, blocking]
depends_on: []
created: "2026-03-23T03:11:53Z"
updated: "2026-03-23T03:11:53Z"
---

## Problem

Tests that mutate environment variables should either run serially or use process-level isolation, since `std::env::set_var` is unsound under concurrent access (hence its `unsafe` marking since Rust 1.83). Instead, tests in `missouri/src/executor.rs` call `unsafe { std::env::set_var("MISSOURI_SANDBOX", ...) }` and `unsafe { std::env::remove_var("MISSOURI_SANDBOX") }` (lines 2359, 2365, 2415-2417, 2458, 2463) with only a comment claiming "test is single-threaded for this env var manipulation" — but `cargo test` runs tests in the same process with the default thread pool, and missouri's own `run_all_paths` uses rayon for parallelism. The save-and-restore pattern provides no synchronization against concurrent test threads reading the same env var.

## Open Questions

- Are these tests actually run in a single-threaded test binary, or does the "single-threaded" comment reflect wishful thinking?
- Would `temp_env` or `serial_test` crates be appropriate, or should these tests shell out to a subprocess to isolate the env mutation?
- Are there other env var mutations in the missouri test suite beyond `MISSOURI_SANDBOX`?

## Why It Matters

Concurrent env var mutation is undefined behavior in Rust. At best this produces flaky test results where `detect_sandbox` reads a partially-written or missing value. At worst it corrupts memory. The `unsafe` blocks are the compiler's way of saying this needs a correctness argument, and the argument provided ("single-threaded") doesn't hold.
