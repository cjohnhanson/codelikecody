---
title: "Short ID prefix for tisket filenames to enable human-friendly references"
status: todo
priority:
assignee:
labels: [tisket]
depends_on: []
created: 2026-03-02T13:54:05Z
updated: "2026-03-05T03:22:00Z"
---

Tisket IDs are slug-ified titles, which means they're accurate but brutal to type and reference in conversation. Something like `clc-dispatch-should-clean-up-stale-worktrees-from-prior-failed-runs-before-dispatching` is fine for machines but unusable for humans.

## Proposal

Prepend a short unique prefix (e.g., 4 alphanumeric chars) to the filename on creation:

```
rjv3-short-id-prefix-for-tisket-filenames.md
```

Then all tisket operations accept either the full ID or just the short prefix:

```bash
tisket issue show rjv3
tisket issue show rjv3-short-id-prefix-for-tisket-filenames
# both resolve to the same issue
```

## Constraints

- **Backwards compatible**: tiskets without a prefix continue to work. Detection is simple — if the first segment before `-` matches `[a-z0-9]{4}` and the rest is a valid slug, it has a prefix. Otherwise it doesn't.
- **Unique prefix**: generated at creation time, must not collide with any existing prefix in the project. 4 chars of `[a-z0-9]` gives 1.6M possibilities — more than enough.
- **No duplicate slugs**: the uniqueness rule on the slug portion (everything after the prefix) still applies. You can't have both `abc1-some-ticket` and `def2-some-ticket`. The prefix adds uniqueness, it doesn't replace existing uniqueness.
- **Prefix is stable**: once assigned, the prefix never changes. Renaming the title re-slugifies the slug portion but keeps the prefix.

## Resolution

- `tisket issue show <prefix>` should resolve unambiguously. If somehow two tiskets share a prefix (shouldn't happen), error with "ambiguous prefix."
- Tab completion / fuzzy matching on the prefix would be nice but isn't required for v1.

## Where it touches

- `tisket issue create`: generate and prepend the prefix
- `tisket issue show/edit/path/close`: accept prefix as ID, resolve to full filename
- `tisket issue list`: display prefix prominently (maybe as a separate column)
- File format: prefix is part of the filename, not stored in frontmatter (keeps it simple)
