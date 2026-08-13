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

A hard review of the full ecosystem, taken 2026-08-12 after the day's changes: gaff shipped, vitrine shipped, almanac reshaped, the nix cutover, and the controlled-English sweep. The review states what is sound, what contradicts the ecosystem's own principles, and what to do first. Each section stands alone so an artifact can transclude it.

## Verdict

The ecosystem is eight repos: six living tools, one library, and one host repo in decline. The new tools are well tested and well documented. The old tools improved today but keep real defects. The largest problems are not in any one tool. They are structural: the host repo has lost its identity, the founding principle of enforcement now has no enforcer, and the tools compose in documentation more than they compose in practice.

## Identity: the host repo has no clear purpose

codelikecody is the ecosystem's name, but the repo now holds the least of the ecosystem. Its living content is missouri. The rest is the clc mothball, clc-sdk, clc-api, clc-web, and the claude-code protocol crate. The README links to tisket/, almanac/, zettel/, and belmont/ as directories. Those directories do not exist in the tree. The links are dead on GitHub. The docs list commands for tools the repo no longer contains.

SECOND CORRECTION (same day, from the divergence-mapping agent): there is no fork, and there was no contradiction. origin/main is a strict ancestor of local main. Local main is 16 commits ahead and 0 behind: the moose removal and the full zs98 split were never pushed. The 5hjq clc commits sit below the merge base in both histories. The first correction's fork claim came from an ancestor check run with reversed arguments. The reflog shows 187 fast-forward pushes and no rewrite.

Consequences: the nix cutover pinned the pre-split origin tip because that was all origin had; one fast-forward push of local main fixes the source of truth, and a pin bump then removes the stale tool binaries naturally. PR #1 is correctly based. The clc "mothballed" status stands; 5hjq was old work.

Also corrected: missouri is moved out as a repo (active, with its own tracker). The split itself never touched the workspace member, so the member and the standalone repo coexist with unknown drift, and the member still holds the docs. Reconciling the two missouri trees is the remaining open item, not the extraction.

Direction: push local main (waiting on the user's word — the trunk-push gate blocked it, correctly). Then bump the co.d pin, move clc-api and clc-web into tisket's orbit or archive them, and decide the codelikecody endgame.

## Testing: strength is inverted against risk

The newest tools have the strongest tests. gaff has 33 unit tests and 15 missouri paths. vitrine has 15 and 5, with a renderer-parity path. almanac has 53 and 6, hermetic, with deterministic git fixtures.

The oldest tools carry the most user data and the least verification. tisket holds every issue in every repo, and its serializer hand-rolls YAML — a known silent-data-loss risk (issue r2lp) — with six unit tests. zettel's suite fails on a nix deprecation warning in its stderr assertions. belmont is the security tool, and it has one missouri path, currently red on a fixture newline.

The risk order is exactly backwards: the tool that guards secrets and the tool that holds all work records are the least tested. Direction: fix the two broken suites first (both are tisketed, both small), then give tisket's serializer a round-trip property test before any new tisket feature.

## Enforcement: the founding principle has no enforcer

Principle two says: make undesired behavior impossible. clc enforced that principle, and clc is off. The trunk guard is gone. gaff blocks nothing, by design and for good reasons. Every repo now depends on convention: no direct pushes to trunk, no unreviewed prose to main. Today showed what convention is worth under pressure — an unreviewed README went to a public main.

Direction: pick the lightest mechanism that restores the load-bearing rules. Native Claude Code permission rules and hooks per repo can block trunk writes without any new tool. A small standalone guard handler is the next step up. Do not grow the guard back inside gaff; that boundary was drawn deliberately.

## Integration: composition on paper, solos in practice

The tools are designed to compose over text streams, and the designs are good. The actual wiring is thin. The almanac skills index is documented as a gaff prime section, but no repo wires it, so the context-decay problem that motivated it (issue irx9) remains unsolved in practice. vitrine's tisket and zettel attach conveniences are tisketed, not built. The co.d skill library — 63 directories — is not under the almanac manifest, so the curation tool curates nothing yet. gaff counts events, and only reminders consume the counts.

Direction: wire one real loop end to end before building anything new. The natural first loop: almanac manifest in co.d governing a few skills, index --md written to a gaff section, refreshed on cadence. That exercises three tools and pays down irx9.

## Distribution: working, with traps

All six binary tools now install from their own repos through co.d. Three traps remain. First, the clc bundle still builds wrapper binaries for tisket, zettel, almanac, and belmont from pinned old revisions, so `clc tisket` and `tisket` can act differently; the hiPrio entries hide the skew instead of removing it. Second, the five extracted repos have CI; gaff and vitrine — built today — have none, so their tests run only on the machines of people who remember to run them. Third, the uncommitted demerit input blocks every co.d lock and rebuild until that repo exists on GitHub.

Direction: remove the wrapper binaries from clc's build; add the standard CI workflow to gaff and vitrine; push or park demerit.

## Security: the scanner has not scanned home

almanac ships a red-flag scanner because public skills are a real injection surface. The co.d library that every session loads has never been through it. belmont's honest disclaimer is good, but the tool with the narrowest margin for error has the weakest tests (see above). vitrine's inbox binds to localhost and validates slugs and JSON; acceptable for its threat model.

Direction: when the co.d library migrates under the manifest, the scanner runs over all 63 skills as a side effect. That migration is the security move, not just the curation move.

## Coordination: parallel sessions now collide

Three collisions happened today: a second session's uncommitted demerit edit broke this session's patches; codelikecody main moved past the local clone mid-session; the co.d working tree carried two sessions' edits at once. The mothballed clc supervisor was the ecosystem's answer to multi-agent coordination, and nothing replaced it. As more parallel sessions run, this class of failure grows.

Direction: near term, treat shared working trees as append-only during parallel work, and put session claims in tisket scratch. Long term, this is the strongest argument for the clc-as-harness revival — but that decision belongs to its own discovery, not to a review.

## Unexamined

mdstore is not cloned locally and was not reviewed. The claude-code protocol crate duplicates envelope parsing that gaff also implements independently; two parsers of one protocol will drift. Both need a look in their own right.

## Priorities

1. DONE for belmont (fixture regenerated, suite green, pushed). Zettel is green in preinstalled mode; the root cause is missouri's sandbox wrapper polluting asserted stderr (tisketed).
2. Push local main (a 16-commit fast-forward; the trunk gate needs the user's word), then bump the co.d codelikecody pin.
3. Wire the almanac-to-gaff skills-index loop in co.d; migrate the library under the manifest and let the scanner run.
4. Restore trunk protection with native permission rules in the repos that need it.
5. Add CI to gaff and vitrine.
6. Remove the tool wrappers from the clc build.
7. Reconcile the missouri docs question; decide the codelikecody endgame.

