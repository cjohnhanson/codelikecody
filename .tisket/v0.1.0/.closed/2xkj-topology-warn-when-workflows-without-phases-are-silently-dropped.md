---
title: "topology: warn when workflows without phases are silently dropped"
status: done
priority:
assignee:
labels: [clc, config, auto]
depends_on: []
created: 2026-04-06T13:22:26Z
updated: "2026-04-06T13:36:51Z"
---

## Problem

topology.rs:238-244 uses filter_map to silently drop workflows without phases. A typo in clc.yaml (e.g. 'phase:' instead of 'phases:') means the workflow disappears with no warning. Workers dispatched with that workflow silently get default_tdd instead.

## Fix

Add eprintln warning in the filter_map when a named workflow has no phases. Something like:
`eprintln!("topology: workflow '{name}' has no phases — skipping")`

## Evidence

```rust
let workflows = self.workflows.iter()
    .filter_map(|(name, spec)| {
        let phases = spec.phases.clone()?;
        Some((name.clone(), config::WorkflowDef { ... }))
    })
    .collect();
```
