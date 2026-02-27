---
title: "Multi-agent workspace orchestration"
status: in_progress
priority:
assignee:
labels: [architecture, feature]
depends_on: [trunk-protection-and-commit-discipline]
created: 2026-02-25T00:00:00Z
updated: "2026-02-27T04:47:56Z"
---

clc already orchestrates single agents via hooks, phases, and tiskets. This
tisket extends that to multi-agent coordination: a coordinator agent on trunk
dispatches work to autonomous worker agents running in isolated workspaces.

## Workspace trait (lives in clc-sdk)

An isolated environment where an agent can work, plus a control interface for
communicating with it.

```
trait Workspace {
    fn create(tisket_id: &str) -> Result<Self>;
    fn send_message(&mut self, msg: &str) -> Result<()>;
    fn recv_output(&mut self) -> Result<Vec<OutputMessage>>;
    fn status(&self) -> WorkspaceStatus;  // running, completed, stuck, permission_denied
    fn permission_denials(&self) -> Vec<Denial>;
    fn destroy(self) -> Result<()>;
}
```

v1 implementation: git worktree + Claude Code child process with stream-json.
Future implementations swap the backing without changing the coordinator loop.

## Agent control: stream-json over piped stdio

Claude Code supports `--input-format stream-json` and `--output-format stream-json`
with `--print`. This gives structured NDJSON in both directions — no terminal
emulation or escape sequence parsing needed.

**Output stream** emits typed JSON messages:
- `system/init` — session info, available tools, model
- `assistant` — streaming content: thinking, text, tool_use
- `user` — tool results fed back to the model
- `result` — final result with cost, duration, `permission_denials[]`

**Input stream** accepts user messages:
```json
{"type": "user", "message": {"role": "user", "content": "follow-up prompt"}}
```

The coordinator reads JSON lines from stdout and writes JSON messages to stdin.
No PTY needed. `std::process::Command` with piped stdio.

**Permission model in stream-json**: permissions are NOT interactive. Denied
tools return an error to the model, which adapts. The `result` message includes
a `permission_denials` array listing every tool that was blocked. The coordinator
reviews denials after the run and can widen permissions for the next invocation.

## Permissions management

Three layers, merged at launch time:

1. **Global defaults** — what any worker gets (`Read`, `Edit`, `Glob`, `Grep`,
   `Write`). Configured in clc settings.

2. **Per-tisket overrides** — tisket frontmatter declares what the work needs:
   ```yaml
   ---
   title: "Fix CI pipeline"
   allowed_tools: [Bash, Read, Edit, Write]
   ---
   ```

3. **Coordinator escalation** — if a worker finishes with `permission_denials`,
   the coordinator decides whether to re-run with wider permissions or flag for
   human review.

v1 starts with `--dangerously-skip-permissions` to get the mechanics working.
Permissions narrowing comes once there's real data about what workers need.

## Workers are hook-governed but coordinator-supervised

Workers are complete clc-managed agents with the full hook stack — phase
enforcement, stop hook, missouri tests, UserPromptSubmit reinforcement,
PostToolUse nudges. The hooks keep workers on track for the common case.

The coordinator supervises between turns:

- **Course correct** — send a follow-up message via stdin if output indicates
  the worker is going off track.
- **Re-prompt on completion** — if the worker stops before work is complete,
  send another message to continue.
- **Adjust permissions** — review `permission_denials` and re-launch with wider
  `--allowedTools` if appropriate.
- **Merge completed work** — when a worker reaches phase=done, run `clc merge`
  from trunk.

The hooks handle enforcement. The coordinator handles judgment.

## Coordinator loop

The coordinator runs on trunk (read-only). Its loop:

1. Check for pickable tiskets
2. Create workspace: `clc pickup <tisket_id>` (creates worktree)
3. Launch worker: spawn `claude --print --input-format stream-json
   --output-format stream-json --dangerously-skip-permissions` in the worktree
   with piped stdio, reader thread buffering JSON lines from stdout
4. Monitor: read `recv_output()`, check `status()`, send follow-up messages
   via `send_message()` if needed
5. On completion (phase=done): `clc merge <tisket_id>` from trunk
6. On failure: log denials/errors, decide whether to retry with adjusted
   permissions or flag for human review
7. Tear down workspace

The coordinator always runs from trunk, so worktree cleanup never fails
(not standing inside the thing being deleted). The coordinator builds and runs
from trunk's `target/`, workers don't need their own.

## Claude Code launch flags for workers

```
claude --print \
  --input-format stream-json \
  --output-format stream-json \
  --verbose \
  --dangerously-skip-permissions \  # v1; replaced by --allowedTools later
  --system-prompt "..." \           # coordinator-provided context
  --model opus \                    # or per-tisket model selection
  --max-budget-usd 5.00 \          # cost cap per worker
  --no-chrome \                     # no browser for workers
  --session-id <uuid>               # deterministic session IDs
```

Key flags:
- `--allowedTools` / `--disallowedTools` — restrict tool access per worker
- `--permission-mode` — `bypassPermissions` for autonomous workers
- `--append-system-prompt` — inject tisket context and coordinator instructions
- `--max-budget-usd` — prevent runaway spend
- `--replay-user-messages` — coordinator can track which messages were processed

## Future implementations

The workspace trait abstracts over the environment. v1 is local git worktrees.
Future backends swap in without changing the coordinator:

- Coder workspace (remote dev environments)
- Docker container
- K8s namespace

## Open questions

- Can multiple workers run in parallel on different tiskets?
- How does error recovery work when a worker gets stuck in a loop?
- Should the coordinator itself be a Claude Code agent (with its own hooks and
  phase enforcement), or a simpler Rust program?
- What's the right cost/budget model for multi-worker runs?
