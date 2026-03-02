---
title: "clc.yaml should configure additional worker permissions beyond the hardcoded baseline"
status: done
priority:
assignee:
labels: []
depends_on: []
created: 2026-03-01T22:09:45Z
updated: "2026-03-02T02:42:15Z"
---

The hardcoded BASELINE_PERMISSIONS in permissions.rs covers the universal set (file ops, search, web lookup, clc/tisket/missouri/cargo, basic shell). But projects may need additional bash commands (npm, pip, docker, make, etc.) or other tool permissions.

clc.yaml should have a `permissions` section where projects declare additional allow rules that get merged into the seeded settings.local.json at dispatch time. Something like:

```yaml
permissions:
  allow:
    - "Bash(npm *)"
    - "Bash(pip *)"
    - "Bash(docker *)"
```

These get appended to BASELINE_PERMISSIONS when seed_baseline() runs.
