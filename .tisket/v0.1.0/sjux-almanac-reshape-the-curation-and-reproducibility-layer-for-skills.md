---
title: "almanac reshape: the curation and reproducibility layer for skills"
status: discovery
priority: 2
assignee:
labels: [architecture, almanac]
depends_on: []
created: 2026-08-12T18:19:11Z
updated: 2026-08-12T18:19:11Z
---

Direction set 2026-08-12: almanac stays and gets used. The skills landscape (zettel ozvq) shows discovery/install/registry solved by npx skills + skills.sh, but with no lockfile, no pinning, no review gate — in an ecosystem where audited skills carry prompt injection at ~36%. Almanac becomes the layer that's missing:

1. almanac.yml manifest: skills declared with pinned source (repo+rev+path | local path | registry name) and content hash; almanac sync materializes exactly that. Cargo.lock for skills; mechanizes the hand-vendoring co.d/skills does today.
2. Review-gated updates: almanac update shows the diff vs the pinned copy, re-pins on acceptance. Nothing changes silently.
3. Provenance + staleness in almanac list (origin, rev, drift) — makes moose-ghost rot visible.
4. almanac index --md for gaff-injected skills-index sections (old clc prime job, recovered as composition).
5. Cedes discovery/search/multi-agent install to npx skills; accepts its source syntax where sensible.
6. Built-ins removed (their consumer, clc workers, is mothballed; the manifest-controlled library replaces them).

Gaff and vitrine repo skills become manifest sources, closing the local-tools-to-sessions gap.

## Open questions
- Manifest home: co.d (the user library) vs per-repo almanac.yml vs both with scopes
- Hash/pin format; whether to reuse npx skills' canonical-store layout for interop
- How sync coexists with nix's read-only management of everything else in ~/.claude
- Diff review UX in a terminal (vitrine artifact with transcluded diffs is the fun answer)

## Scratch Notes

## 2026-08-12 — design v1 (pre-review)

### Manifest (almanac.yml, lives next to the library it governs; co.d gets one)
library: skills
skills:
  - name: gaff
    source: github:cjohnhanson/gaff     # or path:<dir>, or git:<url>
    path: skills/gaff                   # skill dir within source
    rev: <full sha>                     # git sources only
    sha256: <content hash: sorted rel-paths + bytes>

### Commands
- init — empty manifest
- add <source> [--name --path --rev] — fetch, vendor into library/<name>, pin rev+hash, append entry
- sync [--check] — materialize every entry at its pin; --check verifies library matches manifest hashes, exit 1 on drift (CI-able)
- update [name..] [--yes] — fetch upstream head, unified diff (git diff --no-index) vendored vs new, accept => re-vendor + re-pin. Nothing changes silently.
- list — name, origin, short rev, local drift (hash mismatch), upstream drift with --fetch
- show <name>, index [--json|--md] — --md is the gaff section payload (name+description lines)
- remove <name>; docs
- Built-ins deleted: the 19 vendored review skills leave the binary/repo; repo keeps only skills/almanac (its own product skill). Their content already lives in co.d's library.

### Mechanics
- Git fetch: temp dir, git init + fetch --depth 1 origin <rev> (GitHub allows SHA fetch), checkout FETCH_HEAD. Update: fetch default head. Shell out to git (unix, no libgit2/gix dep).
- Hash: sha256 over sorted (relpath, bytes) pairs. sha2 crate.
- Local path sources: vendor-copy like git, hash the same; drift detection makes local dev loops visible.
- clap stays (almanac is not a hook tool; no exit-2 invariant needed).

### Holistic
- gaff: index --md output documented as a .gaff section + cadence recipe (closes irx9's context-decay complaint via gaff's delivery).
- npx skills interop: accept owner/repo shorthand mapping to github:; cede discovery/search/multi-agent install entirely.
- Distribution repair (in scope): no extracted repo has a flake — almanac, tisket, zettel, belmont get flakes (crane template from gaff/vitrine; tisket/zettel already use mdstore as git dep so standalone builds work); co.d adds all four inputs AND bumps the codelikecody pin in the same commit (pre-split bundle currently ships their stale binaries; bump removes them; same-commit avoids bin collisions).
- Ecosystem citizenship for almanac itself: missouri suite (hermetic git fixtures via pinned GIT_*_DATE/name so SHAs are deterministic and manifests byte-diffable), bundled docs refresh, skills/almanac skill, in-repo tisket seeded, LICENSE check.
- vitrine tie (v2, design note only): update --report emitting a vitrine artifact with the diff for review.

### Open risks for review
- rev pinning of local path: sources without git get hash-only pins (no rev) — acceptable?
- update diff UX in terminal for large skills; per-file summary + full diff pager?
- manifest scopes: user library (co.d) vs per-repo manifests — v1 ships manifest-adjacent-to-library, no global registry of manifests.

## Build start
Design v1 under adversarial review (one Opus reviewer: design + hash/injection/interop/determinism angles). Distribution repair running in parallel: tisket/zettel/belmont cloned, flakes generated from the gaff crane template, nix builds verifying in background. mdstore needs no flake (library-only). Sequencing note: co.d gets all new inputs + the codelikecody pin bump in ONE commit to avoid bin collisions with the stale pre-split bundle.

## Review findings — all 12 adopted (design v2 deltas)

1. Hash: framed+versioned (sha256-v1: prefix; per entry u64-len-framed path ‖ kind byte ‖ len-framed payload; raw-byte sort; NFC-normalized paths; deny-list .DS_Store/.git/__pycache__/.almanac-origin; symlinks hashed as (path,target), escaping links refused; case-collision at vendor time = hard error).
2. Managed set: .almanac-origin stamp written inside vendored dirs (excluded from hash); remove/prune refuse unstamped dirs; list shows 'unmanaged' rows explicitly.
3. path: sources become dev: — snapshot-vendored, excluded from sync --check, drift shown as info not failure. Reproducibility claims apply to github:/git: only.
4. add is TOFU: two-phase (stage → red-flag report + SKILL.md → --accept required). Docs say 'trust on first use, pinned and diff-gated thereafter'.
5. Red-flag scanner on add AND update: tool-granting frontmatter, exec bits, non-md payloads, zero-width/bidi/tag-block unicode, base64 blobs, outbound URLs / curl|sh shapes, files outside SKILL.md+references/, NUL bytes (git diff hides binaries).
6. index --md gets --max-bytes with degradation (full lines → name-only → stop+note); writer-step recipe documented with real numbers (current library = 8.5KB vs gaff 4KB default cap).
7. Manifest stores ref beside rev; update resolves ls-remote ref → HEAD symref fallback → hard error.
8. SHA fetch fallback: depth-1 SHA → ref fetch with deepening → full clone; path reported.
9. (folded into 1) NFC + case-collision fail-loud.
10. Missouri hermeticity: GIT_CONFIG_GLOBAL/SYSTEM=/dev/null, -c user/email/gpgsign/autocrlf, epoch +0000 dates, local bare repos via file://, expected SHA asserted in fixtures.
11. Names: manifest name owns dir+addressing; frontmatter mismatch refused at add; index collisions error. Mismatch warnings move to list.
12. diff --no-index exit 1 = difference; empty dirs documented as not carried; manifest self-binding caveat in docs.

## Implementation state (mid-build)

Core landed: hash.rs (framed sha256-v1, NFC, deny-list, case-collision + escaping-symlink hard errors), manifest.rs (Entry with source/path/ref/rev/sha256, dev: detection, atomic save), flags.rs (red-flag scanner, all classes unit-tested), vendor.rs (locate incl. owner/repo shorthand, fetch fallback chain sha→ref→full, resolve_remote with HEAD symref + hard error, copy+stamp), ops.rs (init/add-TOFU/sync±check/update-diff-gated/remove-refuses-unstamped/status-with-unmanaged/index_md-degrading). CLI wired; 53 cargo tests green. Clippy cleanup delegated to a subagent (16 warnings incl. pre-existing in docs.rs/skill.rs — the standalone repo was never lint-clean).

Missouri: old suite had a stale setup path (../../.. from workspace days) and an ignore masking skills/ — both removed; old fixture states kept. New states: workflow (init/TOFU/add/status), curated (tamper caught+restored, hermetic git file:///tmp upstream add+update with pinned dates for deterministic SHAs, unmanaged neighbors safe, index --md budget), flagged (red flags reported, accept records knowingly). index --md CLI flag still to wire after the clippy agent releases src/. Flake staged (crane template).

## Shipped: almanac b9aa38e pushed; co.d cutover in flight

Suite green: 6/6 missouri paths (workflow TOFU, dev-tamper-is-info vs pinned-tamper-fails-check split per design, hermetic git update flow, unmanaged neighbors, index --md budget), 53 cargo tests, clippy 0. Fixture generation caught a live bug: resolve_remote parsed the ls-remote symref line ('ref: refs/heads/main<TAB>HEAD' also ends with HEAD) as the sha. Built-ins removed by subagent (26 include_str dirs gone; repo keeps only skills/almanac product skill); README + cli-reference de-staled; new bundled 'curation' docs page.

co.d: four standalone tool inputs added (almanac/tisket/zettel/belmont -tool), codelikecody bumped to 2c480f1b7 (post-split, no bin collisions). User's in-flight uncommitted demerit input (repo 404s on GitHub) temporarily commented to unblock locking — MUST restore after hms. hms running in background.

## Cutover complete

All four tools now run from their own repos: almanac 0.2.0 (reshaped), tisket 0.1.0, zettel 0.1.0, belmont 0.1.0. The hiPrio entries make them win against the old copies in the codelikecody bundle.

Failure record: two patch scripts did not match their anchor text and did not report it. The user's demerit line sat between the anchor lines. Because of this, the first cutover added inputs but not packages. The fix: all patch scripts now assert each anchor before they write. The demerit input stays in the co.d work tree as an uncommitted user edit. It is not in any commit. A second session works in co.d now; home-manager ran from the committed tree to keep that session's edits out of the build.

Follow-ups: (1) migrate the co.d skill library into almanac.yml, one skill at a time; (2) wire almanac index --md into a gaff section; (3) remove the wrapper binaries from the clc crate when clc retirement lands.
