---
title: "zettel missouri tests fail: extra .zettel dir in initialized state"
status: discovery
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-22T01:50:36Z
updated: "2026-03-23T02:14:13Z"
---

## Problem

The zettel `initialized` missouri test state should contain exactly the files that `zettel init` produces — a `zettel.yml` config file and a `.missouri/` directory for the test harness.

The `initialized` fixture at `zettel/tests/missouri/initialized/` currently contains `zettel.yml` (with `zettel_dir: .zettel`) and the `.missouri/` test config directory. The missouri.yml for this state defines a transition "create note" with a comparator that ignores the `.zettel/` directory (`path: ".zettel/"`, `ignore: true`). However, `zettel init` may now create a `.zettel/` directory as part of initialization, meaning the `initialized` state fixture is missing this directory. When missouri compares the actual state after running `zettel init` against the expected `initialized` fixture, the extra `.zettel/` directory in the actual state causes a diff that isn't accounted for.

The test either fails because of the unexpected `.zettel/` directory in the initialized state, or the ignore rules in downstream transitions are masking the real issue. The fixture needs to either include `.zettel/` or the comparison needs to account for it.

## Open Questions

- Does `zettel init` currently create a `.zettel/` directory, or only `zettel.yml`? The source code in `zettel/src/repo.rs` and `zettel/src/cli.rs` needs to be checked for the exact init behavior.
- Is the `.zettel/` directory created lazily on first note creation, or eagerly on init?
- Should the fixture be updated to include `.zettel/`, or should the transition comparators be adjusted?

## Why It Matters

Missouri test fixtures must exactly match the expected filesystem state. A fixture that's missing a directory the tool creates means the test is either failing or relying on ignore rules to paper over the discrepancy — both of which undermine confidence in the test suite.
