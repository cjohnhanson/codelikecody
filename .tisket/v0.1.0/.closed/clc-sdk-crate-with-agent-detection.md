---
title: "clc-sdk crate with agent detection"
status: done
priority:
assignee:
labels: [architecture]
depends_on: [workspace-restructuring]
created: 2026-02-24T14:52:06Z
updated: "2026-02-25T01:51:46Z"
---

Create `clc-sdk/` workspace crate containing shared traits and utilities for the
codelikecody ecosystem.

## ClcTool trait

```rust
pub trait ClcTool {
    /// Imperative directives for agents. Asserting requirements, not offering
    /// information. Injected by clc at SessionStart and during reinforcement.
    fn prime(&self) -> String;

    /// One-liner summary. Used for periodic context reinforcement.
    /// e.g. "tisket: test-feature (in_progress), 3 open in v0.1.0"
    fn status_basic(&self) -> String;

    /// Complete state dump. Used at SessionStart on feature branches.
    /// e.g. full tisket body, missouri test results, all open issues.
    fn status_full(&self) -> String;
}
```

## Agent detection

`CLAUDECODE=1` is set by Claude Code in the shell environment.

```rust
pub fn in_agent_context() -> bool {
    std::env::var("CLAUDECODE").is_ok()
}
```

When in agent context: output plain markdown.
When in terminal: use termimad or similar for rendered output.

This applies to all output — help, prime, docs, status.

## Crate structure

```
clc-sdk/
  Cargo.toml
  src/
    lib.rs       — re-exports
    trait.rs     — ClcTool trait definition
    agent.rs     — in_agent_context() and related utilities
```

Add `clc-sdk` to workspace members in root Cargo.toml. clc, tisket, and missouri
all depend on clc-sdk.
