---
title: "gaff: unified contextual hook manager for coding agents and git"
status: in_progress
priority: 1
assignee:
labels: [architecture, gaff]
depends_on: []
created: 2026-08-12T15:19:37Z
updated: 2026-08-12T15:27:19Z
---

Build gaff: a standalone unified hook manager for coding agents and git, as a new repo in the codelikecody ecosystem (peer of missouri, tisket, zettel). It replaces clc's hook subsystem; clc itself is being retired (possibly to return later as a coding-agent harness — a separate concern).

## End goal

One binary that owns both hook surfaces:

- **Agent hooks** — all Claude Code hook events route to `gaff hook`; adapters normalize into one event stream
- **Git hooks** — gaff claims `core.hooksPath`; pre-commit/pre-push/post-checkout etc. join the same stream

On top of the unified stream:

- **Handlers** — external commands in gaff.yml with event subscriptions, predicates (branch, file-exists, env, cwd), timeouts, parallel exec. Merge: any block wins, contexts concatenate in config order.
- **Counters** — per-session tallies over the stream (prompts, turns, tool_calls filtered by name, commits). Cadences `every N` / `after N` drive periodic context re-injection ("every 20 tool calls, remind X") to fight context decay.
- **Profiles** — config overlays (handler enablement, cadence overrides, prime sections) with precedence flag > env > session > .gaff/profile > default, and a transition matrix governing which switches an agent may perform on itself vs requiring a human. ProfileChanged is itself an event.
- **Priming** — session-start context assembled from sections (handlers on SessionStart), each section individually refreshable mid-session via counters.

## Non-goals

No workflow phases, no review gates, no orchestration, no workspace management, no language runners or staged-file logic (handlers are dumb commands; wrap pre-commit if wanted).

## Design constraints from first-pass review

1. Git-hook semantics don't fit Allow/Block/Context uniformly: commit-msg/prepare-commit-msg mutate a file, pre-push reads refs on stdin, post-* can't block. Event schema needs per-event capability modeling.
2. Profile transition matrix needs teeth: gaff must guard writes to its own config/state paths (.gaff/**, gaff.yml) via PreToolUse, or the matrix is decorative.
3. gaff.yml in a cloned repo is repo-declared command execution at session start — needs an explicit per-repo trust gate (pre-commit-style install/trust step).
4. "Session" needs precise definition at the edges: git hooks outside agent sessions, subagent events, resume/compaction.
5. One-shot "after N" reminders: fresh process per hook invocation → read-modify-write races on counter state. flock semantics from day one.
6. Injection points must respect per-event output channels (predecessor bug qei8: PostToolUse injection polluted Bash output, workers retried succeeded commands).

## Related prior work

- 8skh (discovery): message-counter-based reminders, per-section cadences, interactive vs autonomous modes — becomes gaff's counter/profile features
- restructure-clc-prime-text-as-skills-with-progressive-disclosure (todo): prime decomposition — becomes gaff's prime sections
- Adversarial design review findings (three independent reviewers: architecture, prior art, operations) to be merged into the plan doc before implementation.

## Success criteria

- gaff repo builds standalone, zero workspace deps on codelikecody
- This repo un-clc-ified and running gaff as its own first dogfood (hooks wiring → `gaff hook`, trunk protection + reminders as gaff config)
- Plan doc at ~/.artifacts/gaff-plan/ reviewed and agreed before implementation starts

## Scratch Notes

## 2026-08-12 — three-review synthesis, scope reshaped to v0.2 (agreed)

Three independent cold reviews (architecture, prior art, operations) all landed. Convergent verdict: gaff as originally scoped was mostly reimplementation. Reshaped and agreed:

**gaff = context-lifecycle handler, not hook manager.** Registers as a handler in Claude Code's native hook config; does not own dispatch (native system has 31 events, matchers, timeouts, 5 handler types — richer than planned replacement). Git surface delegated to lefthook/pre-commit entirely. Blocks nothing in v0; injects only on SessionStart / UserPromptSubmit / PostToolBatch (the batch event is the correct home for counter-driven re-injection — not attached to any tool result).

Core features that survive: counters/cadences (append-only ledger, tool_use_id dedupe, session vs repo state tiers), one-shot future reminders (O_EXCL claim, at-least-once, re-armed on compaction), prime sections on cadence (the one thing native rules/skills can't do — cadence vs condition), profiles as advisory overlays (matrix validated on resolution path with structural identity, not write path).

Key verified facts: CLAUDE_CODE_SESSION_ID exported to Bash subprocesses; CLAUDE_CODE_CHILD_SESSION fragments subagent attribution; clc's adapter field names already stale vs live docs (tool_response→tool_output, prompt→user_input, source→startup_mode); repo config must be data-only (RCE-on-clone otherwise); state lives user-scoped (~/.local/state/gaff), never in-repo (git clean -xdf).

Full findings + dispositions: ~/.artifacts/gaff-plan/ (http://localhost:8080/gaff-plan/).

Next: scaffold ~/Projects/gaff (build plan step 1-2: cargo init, envelope + capability table).

## 2026-08-12 — scaffold done, missouri test design under review

Scaffold at ~/Projects/gaff (staged, uncommitted): event envelope + capability table (9 tests green, zero warnings), hook skeleton holding the exit-code invariant (0/1, never 2). Response shape confirmed from clc adapter: hookSpecificOutput.additionalContext.

Missouri suite designed (scratchpad gaff-missouri-design.md): counting on PostToolUse w/ tool_use_id dedupe, arm-on-threshold vs flush-on-safe-event split, one-shots via gaff remind --after N, compaction re-arm, fail-open badconfig, GAFF_STATE_DIR override so state joins the byte diff. Two adversarial reviewers on the design (coverage/oracles + missouri mechanics) running; tests get written after findings merge, then implementation to green.

## 2026-08-12 — missouri suite written and green, counters/cadences/one-shots implemented

Two adversarial reviews on the test design (coverage + missouri mechanics) reshaped it: full-snapshot chained fixtures, numeric exit-code oracles, PostToolBatch flush branch + negative no-flush branches, literal .gaff/ fixtures (dot- convention is source-only), flat session-dir layout (no empty dirs), jq-based JSON assertions, GAFF_STATE_DIR project-level only, remind session resolution via --session/CLAUDE_CODE_SESSION_ID.

Implemented: config.rs (data-only .gaff/gaff.yml), state.rs (append-only ledger, tool_use_id dedupe, O_EXCL fired claims, pure resolve_root), engine.rs (count/arm/flush split, byte cap with skip-and-mark truncation, compact re-arm), main.rs (hook/remind/status, manual arg parsing to hold the never-exit-2 invariant — clap exits 2 on usage errors).

Result: 11/11 missouri paths, 26 cargo tests, zero clippy warnings (pedantic+nursery). The suite immediately caught a real bug unit tests masked: derived Config::default zeroed max_inject_bytes so the no-config path suppressed every flush — now a named regression test.

Side finding from review: ~/Projects/missouri extraction looks stale (no docs/, no dot- support in source) vs installed 0.2.0 binary. Everything staged in ~/Projects/gaff, uncommitted.

## 2026-08-12 — shipped v0.1.0

Full ship sequence executed:
- Prime sections implemented (inject-all at SessionStart, cadence refresh mid-session, missing-file degradation) — 2 new missouri paths.
- gaff init/check/doctor/docs implemented. init is atomic (temp+rename), idempotent, preserves foreign settings entries, refuses to rewrite invalid JSON. docs bundled via include_str (getting-started, configuration, how-it-works). Product skill at skills/gaff/.
- Payload assumptions verified against live hooks reference: tool_use_id ✓, session_id/cwd/hook_event_name common ✓, compact is a SessionStart matcher value ✓, PostToolBatch exists with documented timing ✓. hookSpecificOutput nesting proven by predecessor's production use.
- 15/15 missouri paths, 33 cargo tests, zero clippy warnings.
- Pushed public: github.com/cjohnhanson/gaff (matching sibling repos). flake.nix (crane) added; co.d flake input + home package added, committed, pushed; hms running.
- codelikecody un-clc-ified locally: 14 clc hook stanzas removed from .claude/settings.local.json (backup at .claude/settings.local.json.pre-gaff), gaff registered on 5 events via nix-profile path. Tracked clc files (clc.yaml, clc.yml, .clc/reviewers) still in tree — removal needs a PR, pending permission. Stale .worktrees/ left untouched (may hold unmerged work).
- Dogfood config .gaff/gaff.yml NOT written — reminder/section text is prompt content, needs approval per repo rule. Draft presented in chat.
