---
title: "apply ASD-STE100 and the prose guidelines across all ecosystem prose surfaces"
status: in_progress
priority: 2
assignee:
labels: [ecosystem, writing]
depends_on: []
created: 2026-08-12T21:02:25Z
updated: 2026-08-12T21:02:25Z
---

Direction 2026-08-12: all prose in the ecosystem must follow ASD-STE100 plus the house prose rules. In scope: code comments, CLI output and error strings, bundled docs, READMEs, product skills, UI copy (vitrine template). Out of scope for now: mothballed clc source prose, clc-web UI (relocation pending). Constraint: behavior does not change. Tests stay green. Missouri fixtures that assert output bytes get regenerated. Quoted epigraphs in READMEs stay as quotations; the prose around them converts.

## Scratch Notes

## Fan-out dispatched

Six agents run in parallel, one per repo group: gaff, vitrine, almanac, tisket, zettel+belmont, codelikecody (branch retire-clc-config; scope = top-level docs, missouri crate, .gaff reminder texts; clc/AGENTS.md excluded). Shared rule set: sentence caps, active voice, imperative instructions, simple words, no idioms, epigraph quotes preserved. Hard gates per repo: clippy 0, cargo tests green, missouri suite green with regenerated fixtures where output strings are asserted. Protected surfaces named per repo: parsed output formats (tisket list table, missouri PASS/FAIL lines, gaff attribution prefixes, vitrine watcher line), identifiers, sentinels. Agents do not commit; review + commit + push happen centrally after each report.

## Sweep complete — all seven repo groups converted

Gates per repo: gaff 33 tests + 15 missouri paths + clippy 0; vitrine 15 + 5 + 0; almanac 53 + 6 + 0; tisket 6 + 27 paths (945 assertions) + 0 (agent fixed 6 trivial pre-existing warnings); zettel/belmont converted with pinned strings verified by hand (their suites have pre-existing defects, now tisketed in-repo); codelikecody top-level + missouri crate converted, workspace build clean, 4 pre-existing illinois failures root-caused (uv init layout drift; appended to the existing tisket with the authors-field trap).

Slop removal: almanac README + curation page lost the npx-skills positioning and the unsourced statistic (pushed, per Cody's direct complaint). Memory rule recorded: no positioning copy in READMEs; user-facing prose holds for review.

Defects found by the sweep, all tisketed in their repos: zettel stderr assertions vs nix deprecation warning; belmont fixture newline; both repos' stale bin shims (fixed in-tree); tisket orphaned docs-list state with wrong assertions; tisket init missing default project; missouri orphaned has-docs state with two title-grepping assertions that can never pass (8k72); missouri writing-tests listed a wrong command (fixed).

Commit state: local commits in gaff, vitrine, tisket, zettel, belmont — NOT pushed, waiting for review. almanac pushed. codelikecody pushed to the retire-clc-config branch (draft PR #1 is the review surface). Pending prompt-content approval: the two .gaff reminder text drafts.
