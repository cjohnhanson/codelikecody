---
title: "ecosystem review 2026-08-12: the hard holistic look"
status: discovery
priority: 2
assignee:
labels: [ecosystem, review]
depends_on: []
created: "2026-08-13T00:07:43Z"
updated: "2026-08-13T00:07:43Z"
---

A hard review of the full ecosystem, taken 2026-08-12 after the day's changes: gaff shipped, vitrine shipped, almanac reshaped, the nix cutover, and the controlled-English sweep. This is version 2. Every claim in version 1 went to an independent checker. Three deep code reads and three cross-repo lenses ran in parallel. This version carries the results. Each section stands alone so an artifact can transclude it.

## How to read the status marks

Each claim carries one mark.

- `[verified]` — a checker reproduced the claim.
- `[corrected]` — the claim was wrong or overstated. The correct statement follows.
- `[new]` — the finding is new in this pass.
- `[open]` — nobody checked it in this pass. Treat it as unproven.

## Verdict

The ecosystem is eight repos: six living tools, one library, and one host repo in decline. The build is clean everywhere. Every green suite that a checker ran is green for real, with one exception, and that exception is on the tool that holds all work records.

Version 1 said the largest problems were structural, not in any one tool. That was half right. The structural problems are real and are confirmed below. But the deep reads found nine defects in the three newest tools that a user can hit today, two of which read files into the model's context or onto the network that no user asked for. The new tools are not the safe part of the ecosystem. They are the part with the newest bugs and the least field time.

The pattern under most findings is the same. Each tool tests the design its author already worried about. Almost nothing tests what the environment does to the tool: parallel calls, hostile input, another machine, another checkout, scale. Every critical defect below lives in that gap.

## Correctness: the new tools carry critical defects

This section is new. Version 1 reviewed the ecosystem and did not read the code.

### gaff

`[new]` A repo config can read any file on the machine into the model's context. `src/engine.rs:148` joins `section.file` onto `.gaff/` and reads it. The join is not normalized. `..` traversal and absolute paths both work. A checker put a canary in `/tmp` and in a parent directory. Both came back inside `additionalContext`. `gaff check` printed `config ok` for both. `README.md:45-47` states the opposite trust boundary in its own words. `.gaff/gaff.yml` is a committed repo file, so this is a supply-chain path into a session.

`[new]` Concurrent hooks corrupt the ledger and lose cadence crossings. `src/state.rs:108` uses `writeln!` on a bare file, which issues several write calls, so append atomicity does not cover the line. `src/state.rs:117-131` counts by reading the whole ledger and then appending, with no lock. Claude Code fires tool hooks in parallel. A checker ran 20 concurrent `gaff hook` calls against a cadence of 20. The reminder failed to arm in 5 of 10 rounds. The ledger picked up interleaved garbage lines, which the reader then discards without a message. The core function of the tool fails about half the time under the normal case.

`[new]` `gaff check` accepts four configs that are dead at runtime and calls each one OK: unknown or misspelled keys (no `deny_unknown_fields`), `max_inject_bytes: 0`, a `/` in an entry name, and a section body larger than the 4 KiB cap. The last one is what a new user hits first. A normal conventions file is larger than one page. The only symptom is the token `[gaff:truncated]`.

`[new]` A one-shot id that already fired cannot be reused. `gaff remind --id ci` overwrites the file, exits 0, and never fires, because the `fired-ci` marker survives. The agent's own scheduling call reports success on a dead reminder.

`[verified]` gaff is live and works. A checker received real injections from the installed binary during the read. The build is clean. 33 unit tests and 15 missouri paths pass.

`[new]` Three README claims are false: a Profiles feature with a transition policy that exists in no line of code, counters over "turns" and tool-name filters that were never built, and the statement that the cargo tests enforce the never-exit-2 rule. `src/main.rs` owns every exit path and has zero tests. No test spawns the binary. The one missouri assertion on exit codes compares a value to itself.

### vitrine

`[new]` The parity guarantee is false, and the live path is an injection sink. `src/render.rs:15` uses comrak defaults, which strip raw HTML and blank dangerous URLs. `assets/vitrine.js:64` uses commonmark.js defaults, which emit both. A checker confirmed the divergence directly on `<div>` and on a `javascript:` link. So the baked page and the served page show different content, which is the exact drift the product exists to prevent. `assets/vitrine.js:83` then assigns that unsanitized output to `innerHTML`, so markup inside a referenced tisket issue runs in the served page, same origin as the response inbox. `README.md:60-61` and `docs/authoring.md:41-45` both state the parity as fact.

`[new]` `vitrine serve` publishes the whole repo root. A checker read `.env` and `.git/HEAD` over HTTP. Nothing limits serving to `.vitrine/` and the referenced markdown. The response inbox accepts a cross-origin simple POST with no Origin check; a checker overwrote `latest.json` with a `text/plain` request. Neither behavior appears in any document.

`[new]` Three silent wrong-content bugs, each confirmed with a repro, each exiting 0. A `#` line inside a four-space indented code block is read as a heading and truncates the section (`src/extract.rs:25`); this project's own docs use indented code blocks. The bake-time attribute reader matches `data-ref` as if it were `ref` and bakes the wrong file (`src/bake.rs:97`). Duplicate heading slugs collide, so `# Alpha` / `# Alpha 1` / `# Alpha` emits the anchor `alpha-1` twice and resolves it to the wrong heading (`src/extract.rs:44`).

`[verified]` The build is clean under a deny-level lint set. 15 unit tests and 5 missouri paths pass.

`[new]` The README status line says the suites cover live transclusion and the bundled docs. Neither is tested at all. The shipped JavaScript runtime is never loaded by any test. The one parity test writes its own front-matter stripper instead of loading the runtime, then compares the renderers on a fixture that contains no raw HTML, so it cannot fail.

### almanac

`[new]` `almanac add` deletes the staged tree before a human can look at it, for git sources. `src/ops.rs:127-133` prints "inspect this path and run again", and the temporary directory drops on return. A checker followed the printed path and got ENOENT. That message is the trust-on-first-use gate the README leads with.

`[new]` `almanac sync` writes unverified content and then reports failure. `src/ops.rs:200-216` fetches, vendors, and only then compares the hash. `src/vendor.rs:175` removes the existing directory first. A checker set a wrong pin, saw `FAIL`, exit 1, and found the new content on disk with a fresh stamp. The good content was destroyed on the way. This contradicts "no change lands without a report".

`[new]` The red-flag scanner misses tool-granting frontmatter in a file with CRLF line endings. `src/flags.rs:100` searches for `\n---\n`. A checker scanned a file with `allowed-tools: Bash(rm -rf /)` in CRLF frontmatter and got `clean`.

`[new]` `add` hand-parses the skill name and keeps the YAML quotes, so a quoted name creates a directory whose name contains quote characters, and every later `list` prints a mismatch warning. `index --md --max-bytes` is not a budget below 78 bytes and degrades per entry rather than in steps, which contradicts four documents and the flag's stated purpose of fitting a gaff cap.

`[verified]` The build is clean. 53 unit tests and 6 missouri paths pass. The hash tests are the strongest tests in the ecosystem. The docs are otherwise accurate and have no dead links, and `docs/curation.md` states the tool's central limit honestly.

## Identity: the host repo has no clear purpose

`[verified]` codelikecody names the ecosystem and holds the least of it. `tisket/` and `belmont/` do not exist in the tree. `almanac/` and `zettel/` exist as empty directories with zero tracked files. `README.md` still links to all four. The links are dead.

`[verified]` Local main is 16 commits ahead of origin/main and 0 behind. There is no fork. Version 1's second correction stands. The push still has not happened.

`[new]` `AGENTS.md` never mentions gaff or vitrine, the two tools that actually run in this repo today. It still describes `skills/` as documentation for missouri, tisket, and clc, and still tells agents that tisket comes from the nix bundle. `ARCHITECTURE.md` carries an honest status banner but still says in the present tense that clc injects prime text on every prompt.

`[new]` `codelikecody/missouri/tisket.yml` is tracked, declares `tisket_dir: .tisket`, and points at a directory that does not exist. A session that starts in that subdirectory gets an empty backlog and exit 0. The tracker's own convention says tisket is the source of truth for cross-session context, and this file silently empties it.

`[new]` The repo root carries untracked and unignored tool state: `.clc/`, `.vitrine/`, `.zettel/`, `skills/`, `belmont.yml`, `.gaff/sections/`, three test binaries, and a file literally named `--full-page` from a misparsed flag.

## Testing: the map was wrong, and the gaps are elsewhere

`[corrected]` Version 1 said testing is inverted against risk, and named tisket and belmont as the least tested. That is wrong on both names. A checker counted.

| tool | unit tests | missouri paths | src lines | lines per unit test |
|---|---|---|---|---|
| almanac | 53 | 6 | 2520 | 48 |
| belmont | 44 | 1 | 1166 | 27 |
| gaff | 33 | 15 | 1924 | 58 |
| tisket | 6 | 27 | 2474 | 412 |
| vitrine | 15 | 5 | 1047 | 70 |
| zettel | 0 | 8 | 1688 | none |

Belmont has the highest unit-test density in the ecosystem, about twice almanac's. Its scrubber and both secret backends are covered. Version 1 measured the new tools with both numbers and the old tools with one number each, which produced the inversion.

`[corrected]` Tisket's hand-rolled serializer is not unverified. Missouri compares whole directories, and tisket ships 45 golden state directories over 27 paths and 263 assertions. Each golden file holds the full serialized frontmatter. The byte output is checked across create, edit, close, reopen, label, tag, due date, body, scratch, and project moves. The real gap is narrower: there is no round-trip property test, so a newly added field that the writer drops would pass, because golden files get regenerated with the schema.

`[new]` The least-tested tool is zettel. It has zero unit tests over 1688 lines and nine source files. Version 1 did not name this.

`[new]` Tisket's missouri suite is currently red: 8 passed, 19 failed, 114 failing assertions. Every failure is the same stderr comparison polluted by missouri's sandbox wrapper deprecation warning. No file comparison fails. This is the same root cause version 1 attributed to zettel alone.

`[new]` Tisket's suite is also green against the wrong binary when it is green. `tests/missouri/.missouri/bin/tisket` is tracked in git as a symlink to `/Users/codyhanson/Projects/codelikecody/target/debug/tisket`, an April binary in the monorepo tisket no longer lives in, and the suite has no build step. A checker copied the tests tree elsewhere and ran it: 27 passed, and the symlink was dangling. With PATH trimmed, 0 passed. The suite has been validating the installed binary, not the working tree, since the split. This is the highest-consequence finding in the review.

`[verified]` Belmont's fixture is fixed and its suite is green. Version 1 contradicted itself on this point and is now consistent.

`[new]` No missouri suite runs in any CI anywhere. gaff, vitrine, and codelikecody have no workflows. almanac, tisket, zettel, and belmont each run `cargo build` and `cargo test` and nothing else. The only end-to-end coverage in the ecosystem, and the only coverage of the shipped binary for gaff and vitrine, runs when a human types the command.

`[new]` No suite in any repo invokes a second tool. Every documented seam has zero end-to-end coverage.

`[new]` `missouri validate` and `missouri list` at the codelikecody root both exit 2 with a bare "No such file or directory". `missouri.yml` still lists `belmont/tests/missouri` and `tisket/tests/missouri`, which the split removed, and omits six suites that exist. The first missing member aborts the run. The whole-repo test entry point has been broken since the split.

Direction: fix the tisket binary link and the sandbox stderr pollution first. Then add missouri to CI in every repo. Then give tisket's serializer a round-trip property test, and give zettel its first unit test.

## Enforcement: one accidental enforcer, and it is switched off

`[corrected]` Version 1 said no repo blocks trunk writes mechanically. One does. `ringer/.claude/settings.local.json` still wires the deployed `clc hook` on every event including PreToolUse, ringer is on main, and a checker confirmed exit 2 and a block message on a Write and on `rm -rf`. That is a leftover, not a decision.

Three facts keep the conclusion intact.

`[new]` It is one repo out of eighteen.

`[new]` Nothing anywhere blocks a push to trunk. The guard's command allowlist contains the bare prefix `git `, so `git push origin main` passes through with exit 0.

`[new]` `CLC_GUARD_OFF=1` is set in this environment by a shell alias. With it set, even ringer's guard returns pass-through. Sessions launched the usual way have no guard at all.

`[verified]` No branch protection exists on any of the 13 remote repos. No rulesets. No git hooks. No deny or ask permission rules in any repo settings file or in the user settings. No CI job anywhere checks prose. With no branch protection, no CI gates a merge in any case.

`[new]` The only live mechanical enforcer in the ecosystem is user-level and enforces a different rule: a pre-tool hook that blocks `git add -A` and `git add .`.

Direction: unchanged. Restore the load-bearing rules with native permission rules and hooks per repo. Add branch protection on the remotes, which is free and blocks the push case the guard never covered. Do not grow the guard back inside gaff.

## Integration: composition on paper, solos in practice

`[verified]` All three sub-claims from version 1 survived a checker that tried to refute them.

`[verified]` The almanac-to-gaff loop is not wired. `.gaff/sections/skills.md` exists in this repo, carries the generating command in its header, and is untracked. `.gaff/gaff.yml` has no `sections:` key and has never had one in any commit. The live binary reports `0 section(s)`. gaff mentions almanac in zero files.

`[verified]` The vitrine attach conveniences exist only as a tisket issue. Neither CLI has an `attach` or `artifact` subcommand.

`[verified]` No `almanac.yml` exists anywhere on the machine. The co.d library is 64 directories, not 63, and none of it is under the manifest, so the scanner has never run over it.

`[new]` The migration is not merely unwired. It is contradicted. `co.d/claude.d/CLAUDE.md` currently instructs the agent to use skills.sh rather than almanac. The instruction and the plan must be reconciled before the migration means anything.

`[new]` The only real code seam in the ecosystem is clc, and it is unplugged. clc links almanac, belmont, tisket, zettel, and missouri as library crates and assembles one prime text from all of them. This repo now registers only `gaff hook`, on five events, where the saved predecessor file registered `clc hook` on fourteen. The commit that deleted `clc.yml` also emptied the source list that feeds the almanac section. The integration code still ships in the binary.

`[new]` The second real seam is mdstore, pinned to the identical revision in tisket and zettel. That one is consistent and healthy. Every other tool declares zero internal dependencies. belmont, almanac, gaff, and vitrine are leaves.

`[new]` clc's prime text tells agents to run `almanac search`, which does not exist and which almanac's own README removed months ago. The same block cites `clc.yml`, a file this repo deleted.

`[new]` The vitrine to tisket and zettel link is a filename coincidence, not an interface. `src/refs.rs:60` hardcodes `.tisket` and `.zettel`, and both stores make those directories configurable. `refs::resolve` is called from exactly one place, the `resolve` subcommand. `bake` freezes a literal relative path that includes the tisket version directory, so closing an issue breaks the artifact.

`[new]` All 12 live transclusions in this repo are byte-identical to their sources today. Nothing can prove that tomorrow. `vitrine sync` has no `--check` flag. almanac has exactly that flag for exactly that problem.

Direction: unchanged in shape. Wire one loop end to end. Resolve the skills.sh instruction first. Add `vitrine sync --check` and run it in CI. Delete or fix clc's prime text, since it now describes commands that do not exist.

## Duplication and drift

`[corrected]` Version 1 said the claude-code protocol crate duplicates gaff's envelope parsing. It does not. That crate types the `--output-format stream-json` NDJSON stream: messages, content blocks, cost, and denials. It never touches hooks.

`[new]` The real duplicate pair is `gaff/src/event.rs` and `clc/src/adapter/claude_code.rs`. Both read `hook_event_name` from stdin and both emit `hookSpecificOutput.additionalContext`. Both are installed right now. They already disagree on a safety rule, not a detail: clc injects on PostToolUse, and gaff refuses to, with a comment naming that as a known bug class. They also write competing handler lists into the same settings file, 14 events against 5. One of them must stop being a hook handler.

`[new]` gaff duplicates the injection rule inside itself. A capability table encodes which events may receive context, and a hardcoded match in the engine encodes it again. Nothing outside the table's own tests calls the table. Eight of gaff's 33 unit tests assert the copy that has no effect.

`[corrected]` Slugify is already deduplicated where version 1 implied it was not: tisket and zettel both re-export mdstore at the same pinned revision. The live duplication is inside vitrine, which ships a Rust and a JavaScript implementation of the same extraction contract. They have already diverged on tab-separated headings and share an identical bug on indented code blocks. The JavaScript half has no behavioral test.

`[new]` missouri exists twice. The standalone repo is version 0.1.0 and last moved in February. The copy inside codelikecody is 0.2.0, carries three modules the standalone lacks, and is the one installed on PATH. The two source trees differ by thousands of lines. The test harness that every repo depends on is the only tool that was never actually split out, and the abandoned copy is the one a person browsing by repo name will find.

`[new]` The nix layer papers over stale copies rather than removing them. co.d's flake carries a comment saying the codelikecody bundle contains old copies of four tools and that a priority function makes the new tools win. Anything that resolves a binary by store path rather than by PATH priority gets the stale copy.

`[new]` Six near-identical flake templates are locked to identical inputs, which is fine. The duplicated version literal is not fine and has already drifted. tisket and zettel declare 0.2.0 in Cargo.toml and 0.1.0 in the flake, and the nix store path proves the flake wins.

## Distribution

`[corrected]` Version 1 said the clc bundle still builds wrapper binaries for four tools. At HEAD it does not. Cargo reports binary targets only for clc-api, clc-web, clc, and missouri. The installed bundle contains the wrappers because co.d pins codelikecody at a revision from 9 April, before the split; the presence of `moose` in that store path proves it. A pin bump removes those binaries on its own.

`[verified]` The skew itself is real and larger than version 1 stated. clc pins all four tools at 3 May revisions, 8 to 10 commits behind. The almanac pin predates the entire curation layer: that revision has no manifest, vendor, hash, flags, or ops module. `clc almanac --help` lists four subcommands; `almanac --help` lists ten. For tisket, zettel, and belmont, the skew is prose only, one source-touching commit each.

`[new]` The remaining defect is the four git dependencies and the four subcommands they feed. No pin bump touches those. The fix is to delete the four variants from `clc/src/cli.rs` and the four dependencies from `clc/Cargo.toml`, or to track the tools' main branches. Version 1's item would have been marked done after a pin bump while `clc almanac` still shipped a 3 May skill layer.

`[verified]` gaff and vitrine have no CI. The uncommitted demerit input still blocks co.d locks.

## Security

`[corrected]` Version 1 called vitrine's inbox acceptable for its threat model. It is not. The inbox has no Origin check, and the server publishes the whole repo root including `.env` and `.git`. Both were reproduced.

`[verified]` The co.d library has never been scanned. 64 directories, no manifest.

`[new]` The scanner itself has a hole: CRLF frontmatter defeats the tool-granting check.

`[new]` The two paths that inject unreviewed bytes into a model's context are the gaff section traversal and the vitrine renderer. Both were shipped today. Both were reported clean by their own validators.

Direction: fix the three injection paths before the migration. Then migrate the library under the manifest and let the fixed scanner run.

## Conventions: eight tools, no shared contract

`[new]` This section is new. A lens compared every surface across all eight tools.

- Root selection has three forms: `--root` with default `.` in four tools, `-C` plus `--config-dir` in missouri, and nothing at all in gaff and vitrine, which read the current directory and cannot be pointed elsewhere.
- Help and version are not a contract. Five tools exit 0 on `--help` and answer `--version`. gaff and vitrine exit 1 on `--help` and have no `--version` at all.
- missouri inverts the exit-code contract: 2 for any error, including a missing directory. Everywhere else 2 means a usage error. This matters because 2 is the blocking code in the hook contract, which is the reason gaff hand-rolls its argument parser. Any hook that shells out to missouri inherits a blocking exit from a missing file.
- Config naming has three schemes, and missouri accepts two of its own, which the six test suites then split on.
- Four error-message shapes for the same condition, with different prefixes, different streams, and only two carrying a remediation hint.
- Strict lints and CI are mutually exclusive. The three repos with deny-level clippy have no CI. The four repos with CI have no lint block, and their CI dropped clippy and fmt.
- missouri has no LICENSE anywhere and no license field. Every other tool ships MIT, in two different copyright variants.
- gaff and vitrine use a different README template with no Related section, so the two newest tools appear in no sibling's cross-link list. Neither is publishable to crates.io, because both omit the standard package fields.
- Six missouri suites use three or more incompatible ways to put the binary under test on PATH. Only one of them, the relative wrapper script used by zettel and belmont, survives a fresh clone on another machine. Two repos track machine-absolute symlinks; one of those is the tisket failure above.
- Four distinct gitignore states, including two repos with no gitignore file at all.

Direction: pick the one right answer for each surface and write it down once, in a single conventions document. Start with the three that cause real failures: the binary-on-PATH convention, missouri's exit codes, and `--help` exiting 0.

## Coordination

`[open]` Version 1 recorded three collisions between parallel sessions and argued they are the strongest case for a harness revival. No checker examined this in the current pass. The direction stands as written: treat shared trees as append-only during parallel work, put session claims in tisket scratch, and take the harness question to its own discovery.

## Unexamined

`[open]` mdstore is still not cloned locally and was not reviewed, though both its dependents pin it consistently. clc-api, clc-web, and clc-sdk were not read. The standalone missouri repo was identified but not diffed for behavior. belmont's code was not deep-read, only counted.

## Priorities

Ranked by what can hurt a user today, then by what makes the ecosystem honest.

1. Close the two context-injection paths. gaff must normalize the section path and refuse anything outside `.gaff/`, in both the engine and `check`. vitrine must match its two renderers on the safe setting and stop assigning unsanitized markup to `innerHTML`.
2. Fix gaff's ledger. One write of a pre-rendered line, and a lock around count-and-append. Add a concurrency test that runs 20 hooks at once and asserts the crossing.
3. Fix vitrine's server. Limit serving to `.vitrine/` and the referenced files. Add an Origin check to the inbox. Document both limits.
4. Fix tisket's test harness. Replace the tracked absolute symlink with the relative wrapper script, add a build step, and confirm the suite fails when the working tree is broken. Then fix the sandbox stderr pollution that has both tisket and zettel red.
5. Fix almanac's two trust-gate defects. `add` must keep the staged tree until the user acts on it. `sync` must verify before it writes, never after.
6. Repair the repo entry points. Fix `codelikecody/missouri.yml` so `missouri validate` runs. Delete or fix the orphan `missouri/tisket.yml`. Ignore or remove the untracked root state and the `--full-page` file.
7. Push local main, which is a 16-commit fast-forward and still needs the user's word, then bump the co.d pin. This also removes the stale wrapper binaries.
8. Delete the four tool subcommands and the four git dependencies from clc, or point them at main. A pin bump does not fix this.
9. Add missouri to CI in every repo, and add CI to gaff and vitrine. No end-to-end test in the ecosystem runs unattended today.
10. Make the validators tell the truth. `gaff check` must reject unknown keys, a zero cap, a path separator in a name, and a section larger than the cap. `almanac`'s scanner must handle CRLF.
11. Restore trunk protection: branch protection on the remotes, plus native permission rules in the repos that need them.
12. Reconcile the skills.sh instruction with the almanac plan. Then wire the almanac-to-gaff loop and migrate the 64 skills under the manifest, which runs the scanner as a side effect.
13. Reconcile the two missouri trees and decide the codelikecody endgame.
14. Write the conventions document and converge the three surfaces that cause real failures.
15. Correct the false claims in the gaff and vitrine READMEs. A tool that documents a feature it does not have costs more trust than a missing feature.
