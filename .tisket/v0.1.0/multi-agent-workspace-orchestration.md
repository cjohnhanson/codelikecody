---
title: "Multi-agent workspace orchestration"
status: discovery
priority:
assignee:
labels: [architecture, feature]
depends_on: [trunk-protection-and-commit-discipline]
created: "2026-02-25T00:00:00Z"
updated: "2026-02-25T00:00:00Z"
---

clc already orchestrates single agents via hooks, phases, and tiskets. This
tisket extends that to multi-agent coordination: a coordinator agent on trunk
dispatches work to autonomous worker agents running in isolated workspaces.

## Workspace trait

Currently "workspace" is hardcoded as a git worktree. The concept should be a
trait — an isolated environment where an agent can work, plus a control interface
for starting an agent inside it.

```
trait Workspace {
    fn create(tisket_id: &str) -> Result<Self>;
    fn start_agent(&self, context: AgentContext) -> Result<AgentHandle>;
    fn destroy(self) -> Result<()>;
}
```

Implementations:
- Git worktree + Claude Code in portable_pty (v1)
- Coder workspace (future)
- Docker container (future)
- K8s namespace (future)
- tmux session (future)

The trait abstracts over both the environment and the agent control mechanism.

## Workers are hook-governed but coordinator-supervised

Workers are complete clc-managed agents. They have the full hook stack — phase
enforcement, stop hook, missouri tests, UserPromptSubmit reinforcement, PostToolUse
nudges. The hooks keep workers on track for the common case.

But the coordinator is not fire-and-forget. It actively supervises:

- **Approve or deny tool uses** — the coordinator fills the "human in the loop"
  role for the worker. Tool use confirmations route to the coordinator rather
  than a human.
- **Course correct** — if a worker is going off track, the coordinator can send
  a prompt to redirect.
- **Re-prompt on premature stop** — if a worker stops before work is complete
  (stop hook blocks, but the coordinator can also re-prompt to continue).
- **Provide context** — the coordinator can answer questions or provide
  additional input when the worker needs guidance.

The hooks handle enforcement. The coordinator handles judgment.

## Coordinator role

The coordinator agent runs on trunk (read-only). Its loop:

1. Pick a tisket
2. Create a workspace (`clc pickup` or equivalent)
3. Start a worker agent in the workspace
4. Supervise: approve tool uses, course correct, re-prompt as needed
5. Collect result when worker completes

## Agent control interface

The workspace trait requires a way to start and control an agent inside the
workspace. For the first implementation:

- Claude Code running in a portable_pty
- Coordinator can send keystrokes to start the agent with context
- Agent runs autonomously under clc hook governance
- Completion is signaled by the agent finishing (phase=done, `clc done` runs)

The control interface supports ongoing interaction — reading worker output,
sending prompts, approving/denying tool use requests. The coordinator is an
active supervisor, not a one-shot launcher.

## Refactoring needed

- Current worktree creation in `clc pickup` should be refactored behind the
  workspace trait
- `clc-sdk` may need the workspace trait, or it may warrant its own crate
- The distinction between trunk agent (coordinator) and workspace agent (worker)
  should be explicit in the type system

## Open questions

- Where does the workspace trait live? clc-sdk? New crate?
- How does the coordinator learn that a worker finished? Polling state files?
  File watcher? The pty closing?
- Can multiple workers run in parallel on different tiskets?
- How does error recovery work when a worker gets stuck?
- Should the coordinator be able to dispatch to remote workspaces (Coder, k8s)
  from the start, or is local-only fine for v1?
