---
title: "Arbitrary key:value tags on tisket issues"
status: in_progress
priority: 3
assignee:
labels: [tisket, feature]
depends_on: []
created: 2026-02-22T17:22:32Z
updated: "2026-03-22T02:15:24Z"
---

## Summary

Add a `tags` field to issue frontmatter — a freeform `HashMap<String, serde_yaml::Value>` for project-specific structured metadata. Labels stay as the flat categorization primitive; tags are the richer key:value layer.

## Frontmatter

```yaml
title: "Fix the thing"
status: todo
labels: [clc, bug]
tags:
  estimate: 4
  sprint: "2026-Q1"
  severity: critical
```

`tags` defaults to `{}` when absent. Tisket round-trips unknown tag keys without dropping them.

## Querying

`tisket issue list --where key=value` filters issues where `tags.key == value`. Multiple `--where` flags AND together.

`tisket search` also gains `--where` support.

## Scope

- Add `tags: HashMap<String, Value>` to `IssueFrontmatter` with `#[serde(default)]`
- Round-trip preservation (read, modify other fields, write back — tags survive)
- `--where key=value` filter on `tisket issue list`
- `tisket issue edit --tag key=value` to set a tag, `--untag key` to remove
- Display tags in `tisket issue show` output

## Out of scope (follow-up tisket)

- JSON schema validation of tag keys/values at the project level
- Tag value type coercion (everything is serde_yaml::Value for now)
- Label groups / mutual exclusivity (Linear-style)

## Also

`tisket issue list` should gain `--label` filtering — the coordinator already supports it but the CLI doesn't expose it.

## Scratch Notes
