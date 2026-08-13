---
title: "mettle: evidence of the operator's worth, read from the demerit record and the environment"
status: in_progress
priority: 2
assignee:
labels: [architecture, discovery, mettle]
depends_on: []
created: 2026-08-13T01:12:06Z
updated: 2026-08-13T01:18:41Z
---

Build mettle: the tool that shows the power and the importance of the human operator, with evidence for two readers. The operator reads it as recognition: what only you brought, accumulating over time. The boss reads it as legible value: what this person caught, taught, and built that the model and the harness did not. The register is human and warm, not forensic. The deliverable is closer to a brag document with receipts than to an audit.

The substrate is three books. The flow book is the demerit store: the corrections, one event per moment of human judgment. The stock book is the standing operator artifacts: rules files, memory, skills, hooks, fixtures, and the code history, with provenance from git. The attribution book decomposes agent transcripts into operator spans, harness spans, and model spans, over the claude-code protocol crate, with gaff events as the consumption signal. A character share is a floor, not a verdict. The strong evidence is the ablation: the same task with a bare harness and with the operator's full stock, diffed missouri-style; the delta is the operator, measured.

The group language turns human: 'only you' (no rule can hold it yet), 'teachable' (it occurs again; a rule can take it), 'taught' (a rule runs it now). demerit stays a lean log and never becomes a platform; mettle reads demerit and the environment, and writes the story.

Open problem, the hardest one: the boss-reader needs outcome language ('caught a premature deploy claim'), and the record holds tool language ('severity 1, premature-done'). What translation is honest without a human editing every word? Second open problem: what makes a body of per-task evidence add up to a career-level claim instead of a pile of anecdotes?

## Scratch Notes

## 2026-08-12 — v1 build start, scope locked

Goal set: a converged v1. Scope: mettle report (the story with receipts, markdown + --html), mettle stock (the operator inventory with git provenance), mettle attribute (the transcript span floor over the claude-code crate), mettle docs. Group language: only-you, teachable, taught. No demerit code dependency: the store format is the contract, and mettle parses the markdown itself via mdstore. No LLM translation in v1: the receipts are the operator's verbatim words. New repo at ~/Projects/mettle, crane flake, STE prose, cold review before done. GitHub creation stays gated.
## 2026-08-12 — converge-ship phase 1, after correction qp6m

The build stopped after one module, because the scope had not converged (correction qp6m, the third patched-foundation event). converge-ship runs now. The plan is at ~/.artifacts/mettle-plan/ with the stories for the operator, the manager, and the record, the command surface, the report sketch, the three-books table, the v1 scope fences, and four open calls in the form: one report vs --for flag, editorial vs terminal HTML look, attribute in or out, roots flag vs config file. One cold reviewer reads the plan now (product angle, sonnet). The build resumes after the review findings and the form answers.
## 2026-08-12 — plan round 1 applied

The cold review returned 1 blocker, 7 majors, 4 minors, and all twelve got applied; none were rejected. The blocker: git authorship cannot separate operator-typed words from agent-typed words that the operator committed, so the stock claim is now stewardship (maintained, revised, approved), with Co-Authored-By trailers marking the agent's part where they exist, and the report states the limit itself. The majors: a consent model (the report addresses the operator; sharing is the operator's act; the tool sends nothing), time order for the receipts instead of a severity-as-expertise claim, tool output out of the attribution denominator with its volume printed separately, a designed thin-record state, the self-attested limit stated with the store's git history as the integrity trail, the art-directed HTML fenced out of v1 in favor of clean minimal HTML, and a transcript default for attribute (newest transcript for the current repo). The minors: the grouping algorithm specified in the plan, a single-operator fence, the docs command tied to the family convention, and manual --since acknowledged as a decision. The plan at mettle-plan is revised, and the review log records the round. The revision removes surface rather than adding it, so no second plan pass. The form stays open for the user's four calls.
## 2026-08-13 — phase 2 complete, one honesty bug caught live

The core is built and committed: flow (the store reader and the three groups), stock (the scanner with the stewardship claim and Co-Authored-By marks), attribute (the transcript shares), report (markdown and minimal HTML with a closed markdown subset and escaping), cli, docs. 15 tests, 0 warnings. The live run against the real record works: 11 moments, 2 only-you, 8 teachable, 1 taught. The live attribute run caught an honesty bug before the QA pass: harness injections (system reminders, hook context) ride in user messages and were counted as operator words, which made the number an overcount instead of a floor. A harness bucket now separates them; the session share moved from 64.3% to the honest 54.7%. A lock gotcha repeated from demerit: the fresh lock held serde_yml 0.0.12 and 0.0.13 side by side, and the dedupe fixed the mdstore build. Phase 3a next: one QA subagent exercises the CLI and the HTML cold. The plan form answers are still open.
## 2026-08-13 — QA round applied, v1 at the finish line

The QA reviewer exercised everything and found 1 blocker, 3 majors, 1 minor; all five are fixed and verified, and one more honesty bug got caught during the build itself (the harness bucket). The blocker was the worst possible place: the HTML escaped lines before the quote check, so the receipts never rendered as blockquotes in the one format meant for a manager's eyes. Fixed; the check runs on the raw line now, with a regression test. The other fixes: the teachable threshold is shared so no correction can vanish under --min 1, the harness category and denominator rule are documented, the plan's severity marks print in every entry, and the stock command speaks in the correct number. 18 tests, 0 warnings, amended commit 2eeb0e0. The review log in the plan carries both rounds. The nix build verification runs. The one open convergence input is the user's four form answers at mettle-plan.
## 2026-08-13 — the technical side of v1 is closed

The nix build is green after the flake fix: the sed-derived flake had kept demerit's wasm preBuild, the clean flake replaced it, and the nix binary answers with its version. The gate check passed: cargo build --workspace has 0 warnings, and the 18 tests pass. The amended commit is 1aa3c9f. The real report is published at the artifact server under mettle-report, with the receipts as rendered quotes and the four repositories in the stock. The plan carries both review-round logs. The one open convergence input is the user's four form answers at mettle-plan: the audience mechanics, the HTML look, attribute in or out, and the roots source. The build followed the plan's recommendations, so the answers either confirm or redirect.
