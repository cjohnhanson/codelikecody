---
title: "demerit: record human corrections to separate commoditized agent work from human-led work"
status: discovery
priority: 2
assignee:
labels: [architecture, discovery]
depends_on: []
created: 2026-08-12T17:33:04Z
updated: 2026-08-12T20:38:24Z
---

Build demerit: a tool that records each correction that a human gives to a coding agent. The intent is not a failure tracker. A correction is a record of the value that only the human added: the judgment, the knowledge, or the taste that the agent did not have. The record separates the work that is a commodity from the work that is human-led. One example gives the idea: a plumber repairs a pipe in 20 minutes, and the price pays for the 10 years in which the plumber learned which pipe and which repair. The corrections are those years.

The store is one global git repository of markdown events (mdstore). Each event has labels, a severity, the repository, the session, the harness, the agent, and the optional rule: and skill: fields. The read commands find the corrections that occur again: log, tally, recur, and prime. The triage in recur has two readings. A correction that becomes a rule or a skill becomes a commodity. A correction that never becomes a rule shows the work that stays human.

The tool is multi-harness. The core is a CLI, markdown files, and text output. Claude Code is the first detection adapter. The skills/ directory uses the agentskills.io format. demerit serve gives a local web UI (axum and Leptos). All prose obeys ASD-STE100.

## Scratch Notes

## 2026-08-12 — named

demerit. Fits the register (tisket/gaff/belmont): a demerit is a recorded mark against conduct — small, countable, accumulating into a record. Store convention: global git-tracked store, repo: field per event (decision pending confirmation, leans global).
## 2026-08-12 — serve, skill context, /demerit skill (discussion)

Three additions from discussion:
- **demerit serve** — local web UI over the store: correction log, counts, recurrence views. Rust-WASM frontend per ecosystem precedent (clc-web is Leptos CSR served as static files by an axum API; user said 'yew app or something' — stack call pending).
- **skill: field** in the event schema, alongside rule:. Triage matrix falls out: has rule: → enforcement problem (gaff); has skill: → skill rot (skill-improvement flow); has neither + recurs → promotion candidate (new rule/skill).
- **demerit ships a skill** (skills/ dir, per ecosystem convention): /demerit in Claude Code scans the current conversation for correction moments and logs them. This is the capture-ergonomics answer — retrospective agent-assisted capture beats asking the human to log anything. Pairs with a gaff cadence reminder to run it.
## 2026-08-12 — multi-harness requirement (discussion)

demerit must not assume Claude Code. Core stays harness-agnostic (CLI + plaintext files + prime-to-stdout); harness specifics live at the edges:
- Schema: separate harness: (claude-code, codex, opencode, …) from agent: (model). Both inferred when possible.
- Capture skill: skills/ already uses the agentskills.io format, which multiple harnesses discover — one skill, many harnesses.
- record inference: per-harness env/transcript detection as thin adapters inside record, not separate binaries.
- Priming: demerit prime prints text; gaff injects it for Claude Code, other harnesses use their own session-start config surface. demerit knows nothing about any of them.
## 2026-08-12 — harness abstraction layer (discussion)

What clc still uniquely holds: the claude-code protocol crate + the Agent-as-interface idea (clc-sdk::Agent), so nothing touches Claude Code directly. The trait as written is the DRIVE half (build_start_command/build_resume_command — orchestration, clc-revival territory). demerit needs the INTROSPECTION half: session detection (in-session? harness? model? session id? transcript path?) + transcript protocol types (claude-code crate). demerit's record-inference adapters should consume a shared harness layer rather than hand-rolling detection privately — demerit is the second consumer that forces the abstraction to become real, with a second harness impl. Related existing issue: ak7m (move ClaudeCodeAgent into claude-code crate). Parallel to gaff taking clc's hooks half; this is the harness half.
## 2026-08-12 — decisions locked (form response)

Form submission: taxonomy=emergent (free labels, harden later), store=global (~/.demerit git repo, repo: field), frontend=leptos, severity=keep (optional 1-3, default 2, recur weights). No notes. All open calls closed — build starts. Sequencing lean recorded: v1 depends on claude-code crate for protocol types, minimal detection; harness-crate extraction waits for the second harness impl.
## 2026-08-12 — build started

Tasks tracked in-session. ~/Projects/demerit initialized (git, MIT license from gaff, crane flake to follow). Verified in-session: CLAUDE_CODE_SESSION_ID and CLAUDECODE env vars exist → v1 inference needs no transcript parsing (harness=claude-code when CLAUDECODE set, session from CLAUDE_CODE_SESSION_ID; agent/model has no env var, passed by flag/skill). mdstore API confirmed (Document<T> parse/serialize, slug prefix gen, Selector) — dep pinned to rev c74fde4. Design call made during build: prime emits pure data lines (counts, keys, last-seen), no imperative framing — injection framing belongs to the consumer (gaff.yml, user-owned), which keeps prime output out of prompt-content territory.
## 2026-08-12 — core CLI built and verified

~/Projects/demerit: record/log/tally/recur/prime/docs all working, 25 unit tests green, zero warnings. Smoke-tested end to end against a scratch store from inside this session — inference verified live (repo from git toplevel via gix::discover, harness=claude-code + session id from env, flags win over inference). Gotcha recorded: serde_yml 0.0.13 breaks mdstore rev c74fde4 (from_str bound change) — pinned =0.0.12 like tisket. prime emits pure data lines (count× key — last date — latest summary), framing left to the consumer. Remaining: serve (Leptos+axum), skill draft (approval-gated), ship (permission-gated).
## 2026-08-12 — serve built and QA'd visually

demerit serve done: axum JSON API (/api/events, /api/tally, /api/recur) + Leptos 0.8 CSR frontend (Recurrence/Log/Tally tabs), built without trunk — cargo wasm build + wasm-bindgen-cli direct (pinned wasm-bindgen =0.2.121 to match nixpkgs wasm-bindgen-cli), dist embedded into the binary via include_dir (build.rs creates empty dist so plain cargo build works; empty dist → API-only fallback page). 28 tests green, zero warnings. All three views verified in browser via agent-browser screenshots. flake.nix written (crane, dual-target toolchain, UI in preBuild) — nix build verification running. Repo staged, not committed (review gate). Remaining: skill draft approval, ship permission.
## 2026-08-12 — ship in progress

Skill approved and committed. nix build green end to end (wasm UI built in sandbox, release tests pass, binary verified). Initial commit 284b12d on ~/Projects/demerit main. gh repo create blocked by permission classifier — user running it directly. Next after repo exists: co.d flake input + home package (mirror gaff/vitrine), commit co.d, push, hms.
## 2026-08-12 — direction change before ship: ASD-STE100 for all prose

Ship held. New standing directive: all prose in the repo obeys ASD-STE100 (Simplified Technical English), code comments included. Not via a skill — the standard applies directly (user asked why a skill at all; answer: no skill needed in-session, and skill references stopped). Two demerits recorded in the real store during this exchange, the first real events: lnpr (claimed STE compliance in a reply that used an em dash and an idiom). Rewrite in progress: README, both docs, SKILL.md done; code comments, CLI help strings, error messages, UI strings in progress. Triage strings and the web UI 'rule / skill' header change too (em dash and slash are not STE). Commit will be amended before push.
## 2026-08-12 — STE rewrite complete, commit amended

Directive is now firm: all communication and all prose in code obeys ASD-STE100. The full rewrite is complete: README, the two doc pages, SKILL.md, all doc comments, the CLI help text, the error messages, the triage strings, the web UI strings, the build comments, and the commit message. The triage strings changed, and the tests changed with them. The prime and recur output formats have no em dash now. The build has 0 warnings and 28 green tests. The UI dist was built again. Commit 13d501e is the amended initial commit. One repair during the rewrite: an Edit in cli.rs made a duplicate block, and sed removed lines 160-299. The ship still waits for the gh repo create command from the user.
## 2026-08-12 — intent reframe (correction 63mx), UI polish

Correction from the user, recorded as demerit 63mx: the tool is not a failure tracker. A correction records the human contribution, and the record separates commoditized agent work from human-led work. The plumber example holds the idea: the price pays for the years of knowledge, not for the minutes of the repair. The issue title and the issue body now hold this frame. The README, getting-started, and SKILL.md were rewritten around it. The triage keeps two readings: a correction that becomes a rule becomes a commodity, and a correction that never becomes a rule shows the judgment that stays human.

Also complete: the ui-skills.com pass (ibelick/baseline-ui via npx ui-skills). Changes: tabular-nums on data, truncation on the summary column, static skeletons for the load states, empty states with one clear next action, and aria-labels on the input and the select. The views were checked in the browser. Cargo description and the pbcopy repo-create command carry the new frame next.
## 2026-08-12 — the UI now carries the new intent

The Contribution view is the default tab. It shows the three groups: human-led (no rule, does not occur again: the judgment that stays human), codifiable (occurs again, no rule: a new rule can make it a commodity), and codified (a rule or a skill exists). The split logic is src/contribution.rs with tests, the endpoint is /api/contribution, and the CLI command is demerit contribution (CLI-first parity). The log shows the correction text through a details element, because the correction is the human contribution. The event type gained happened() and correction() section readers. 35 tests pass, 0 warnings, the views were checked in the browser, and the CLI check against the real store put the two real session demerits in the human-led group. Amended commit: e467cc2. The ship still waits for the gh repo create command from the user.
## 2026-08-12 — the UI was rebuilt from first principles (correction plbp)

Two more corrections from the user, both recorded in the store: q89s (the UI had low visual quality) and plbp (the method patched a wrong foundation two times instead of a design from first principles). The event 63mx also got an STE fix in its correction text, and t0bc records that failure class. The ste-violation label now occurs two times, so it shows as codifiable: it deserves a rule.

The new foundation: the UI answers one question, namely what the human brought that the agent could not do. It is one document, not a dashboard. The tabs, the log view, the tally view, and the recurrence view are gone from the web UI, because the CLI already gives those query surfaces. The page reads from top to bottom: the account strip with the four figures, the human-led contributions in large serif with red ink, the codifiable group under its labels, and the commodity tail as compressed lines. The correction text is the primary content of each entry, and the agent error is the context line. The look is a conduct ledger: aged paper, ink, a typewriter face for context, and one SVG grain tile. The frontend-design skill guided the pass. 35 tests pass, 0 warnings, and the running server shows the real store. The screenshots confirm all three sections. The ship still waits for the gh repo create command from the user.
## 2026-08-12 — third UI pass, from register devices (correction ki9a)

The second pass was also generic: a centered column, a KPI figure row, and a grain overlay. Correction ki9a records it with the design label, and the design label now occurs two times, so the register itself marks the class as codifiable. The third pass builds on four devices that belong to a register book: the severity as demerit marks (three, two, or one ✗), a red margin rule that runs the full page height, entry numbers with dates and marks that hang in the margin (№ 1 is the first correction ever), and the account at the foot with dot leaders. The grain and the figure row are gone. Small screens fold the margin above the entry. The screenshots confirm the top and the foot. 35 tests, 0 warnings, amended commit on main. The server on port 4242 shows the real store. The ship still waits for the gh repo create command from the user.
## 2026-08-12 — fourth UI pass, with the machinery on (corrections 9wia, l1sj)

Two more recorded corrections: 9wia (the third pass was slop in the colors, the font, and the IA; the design label has three events now) and l1sj (the design and review machinery went unused; rule use-review-and-qa-skills). The method changed. The taste skills came from ui-skills (taste-skill, emil-design-eng, minimalist-skill; design-taste-frontend 404s on the CLI). The taste skill names the exact failure: the warm-paper, oxblood, and espresso palette of passes two and three is on its banned hex list, and serif-as-creative-default is its top AI tell. The dial read for a records tool: low variance, low motion, medium-high density, terminal family. The fourth pass: near-black ground, zinc neutrals, one accent on the mark, monospace only, weight-and-color hierarchy, fixed left rail with the account, dense entry list, no cards, no texture, no animation. Two fresh-eyes subagents review the page cold before the user sees it. The first two subagent runs failed on a Fable safeguard false positive; the retries run on sonnet.
## 2026-08-12 — the build failure that sent reviewers to the old page

The two cold reviews came back with a surprise: they reviewed the register page, not the terminal page. The wasm build had failed with two E0382 borrow errors, the output redirect hid the failure, and the binary kept the old embedded UI. Lesson recorded: never silence the output of a build step in a chain. The reviews still earned their cost. A cold reviewer independently called the paper-and-serif register an AI-template trope, which confirms the user's verdicts from outside the conversation. Transferable findings, now fixed in the terminal build: the dim text failed AA contrast by computation (fg-dim lightened to 6:1), and the severity marks confused a cold reader (a legend sits in the rail now). The borrow errors are fixed, the served CSS is confirmed by curl, and a fresh cold review of the real page runs now.
## 2026-08-12 — cold-review loop closed, commit amended

The fresh reviewer of the real terminal page verified the palette by computation (all AA), praised the restraint and the :target anchor feedback, and found three faults: the codified section broke the shared entry format, the accent on the human count had no legible rule, and the mobile chrome was five elements deep. All three are fixed: codified renders the same entry unit under rule keys, the accent belongs to the marks alone, the meta has one slot per field with the action on its own line, and the legend hides on small screens. Amended commit 66ecb09. 35 tests, 0 warnings. The register shows 8 corrections: 2 human, 5 codifiable, 1 codified. The ship waits on gh repo create from the user.
## 2026-08-12 — product thinking round

The product document is at ~/.artifacts/demerit-product/ (localhost:8080/demerit-product/). The intent statement: demerit answers 'what did the human bring that was not fungible' with evidence; the plumber gives the price logic, and the refinery gives the process logic. The central tension: the human-led group is a trophy case, and the codifiable group is a work queue that should trend to zero. Stories grouped as capture, read, promote, and prove. The one structural addition proposed: a rules ledger in the store (slug, created, covers, source). It makes the codified group computable, turns covered corrections after the rule date into detected enforcement failures, and keeps events immutable. Feature map: v0.2 = ledger + promote + enforcement detection + UI filters; v0.3 = trend + period export + second harness adapter. Open calls in the form: ledger yes/later, promote targets, trend timing.
## 2026-08-12 — intent deepened: operator attribution (discussion)

The user widened the intent: the main goal is to separate model+harness from the operator, and to attribute as much performance as the evidence supports to the operator. The corrections are one source. The environment counts too: AGENTS.md, memory, skills, hooks, the code history. An agent is a loop of inference calls, and the content of each call decomposes into operator spans, harness spans, and model spans.

The synthesis that landed: flow and stock. A correction is the flow. The environment is the stock, and the plumber's ten years are the stock. Codification transfers flow into stock. The product becomes an operator's ledger with three books: flow (the corrections, built), stock (the operator inventory with git provenance; the rules ledger is its first table), and attribution (the per-session span decomposition over the claude-code protocol crate, with gaff events as the consumption signal). The honest floor claim: 'this fraction of what the model read, the operator wrote.' The product document at demerit-product got the new intent section, the three-books table, an Attribute story group, and v0.3 additions. The form stays open.
## 2026-08-12 — the wider intent got its own tool and name: mettle

The attribution instrument split from demerit and took the name mettle, the user's pick from the human-word candidates (knack, mettle, handiwork, hunch, savvy). The register correction that led here: the goal is to show the power and the importance of the human operator, with evidence for the operator and for their boss. Recognition, not litigation; a brag document with receipts, not an audit. demerit stays the lean flow log. mettle reads the demerit store, the stock (environment artifacts with git provenance), and the transcripts (span attribution via claude-code, gaff as the consumption signal), and its strong evidence is the missouri-style ablation. Group language turns human in mettle: only-you, teachable, taught. Discovery issue filed for mettle in this tisket. Two recorded open problems: the honest translation from tool language to outcome language, and the aggregation of per-task evidence into a career-level claim.
