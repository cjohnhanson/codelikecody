---
title: "belmont: scrubber boundary buffer has fragile strip_suffix fallback"
status: discovery
priority: 2
assignee:
labels: [belmont, correctness, security]
depends_on: []
created: 2026-03-24T12:41:27Z
updated: "2026-03-24T12:41:36Z"
---

## Problem

The streaming scrubber in `belmont/src/scrub.rs` uses a `strip_suffix` approach
to split scrubbed output into emittable prefix and retained tail. When
`strip_suffix` fails (because the scrubbed tail text appears multiple times in
the scrubbed full text, or replacements in the prefix change what the tail looks
like), the fallback clears the buffer entirely and emits everything. This drops
the boundary guarantee — a secret starting in the emitted portion and continuing
into the next chunk would leak.

The current tests pass because they cover simple cases. Pathological inputs
(secret values that are substrings of `belmont://` references, multiple
overlapping secrets near chunk boundaries) likely break the invariant.

Consider replacing the string-replacement approach with Aho-Corasick streaming
matching, or restructuring to track original-to-scrubbed byte position mapping
so the split point can be computed correctly.
