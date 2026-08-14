---
title: "artifact management for the ecosystem — interactive artifacts alongside plaintext for tisket and zettel"
status: in_progress
priority: 2
assignee:
labels: [architecture, discovery]
depends_on: []
created: 2026-08-12T16:52:07Z
updated: 2026-08-12T17:31:32Z
---

Plaintext markdown is the ecosystem's storage layer, but some deliverables want to be interactive HTML: plans with diagrams and review findings, side-by-side comparisons, forms that round-trip decisions back to files agents can read. Today that lives in personal infra (~/.artifacts + Caddy + art-backend in co.d), disconnected from the repos the work belongs to. Explore an ecosystem answer: repo-scoped, git-tracked artifacts attachable to tisket issues and zettel notes, served locally, with a response inbox pattern for round-trips. Open questions: extend mdstore (it is a parsing library, not a server — likely wrong home) vs net-new tool; storage layout (.tisket/artifacts/<issue-id>/ vs top-level); linking convention in frontmatter; how serving works across many repos; whether the co.d art-backend gets subsumed. Prior art: the gaff plan doc workflow that motivated this.

## Scratch Notes

## 2026-08-12 — transclusion requirement (from discussion)

Core requirement added: artifacts reference sections of canonical markdown rather than duplicating them — written once, rendered wherever.

Design direction:
- Addressing: tisket:<id>#<anchor>, zettel:<id>#<anchor>, file:<path>#<anchor>. Heading slugs by default; optional explicit anchors for retitle-stable references and for list items that decisions attach to.
- Section extraction (doc + anchor → markdown span) belongs in mdstore — it's a markdown-library concern, and tisket/zettel get --section for free once it exists.
- Serve-time resolution, not build-step: edits to the md are live on refresh. Likely mechanism: /api/md/<ref> endpoint + a small <md-section> web component that fetches and renders client-side; broken refs degrade visibly instead of erroring the page.
- Reference direction is one-way, md → artifact. The artifact never holds its own copy of content, so drift is structurally impossible.
- Response-inbox entries should record decisions against stable refs (issue#anchor), so recorded judgments survive list reordering.
- Write-back (artifact form edits the md directly) explicitly staged OUT of v1 — responses land in the inbox and an agent applies them to canonical text.

## Portability requirement (discussion)

No HTML-native transclusion exists (HTML Imports dead, iframe wrong for fragments). Portable design instead:
1. Resolve tisket:/zettel: refs to RELATIVE PATHS at write time — committed artifacts fetch co-located md files, so any static server works with zero custom backend. Client-side anchor extraction + inlined ~15KB md renderer keeps the file self-contained.
2. Baked fallback in <template data-ref> for file:// / detached sharing — derived copy, mechanically regenerated (sync command or pre-commit hook), superseded by live fetch when served. Snapshot-as-of-last-commit semantics for shared copies.
3. Custom server = enhancement only: response inbox, cross-repo refs, optional SSR.
Locked consequence: in-repo refs never depend on a resolver → artifacts need a fixed in-repo directory so relative paths to .tisket/.zettel are stable.

## Web components as the chassis (discussion)

Browser-standards stack, no framework:
- Custom element <md-section ref>: fetch relative path, extract anchor, render. Undefined-element fallback is legal HTML.
- Declarative Shadow DOM for the baked snapshot: <template shadowrootmode=open> renders during parse with NO JS — upgrades tier 0 (file://, scripts blocked) to full display. JS hydrates with live fetch when available.
- Styling: light DOM for transcluded content (inherit artifact typography); shadow only for component chrome (unresolved-ref badge, refresh state).
- Form-associated custom elements (ElementInternals) for decision widgets — native form participation, submitting stable anchors to the response inbox.
- Caveat: bake path and hydrate path are two md renderers (comrak vs inline JS). Pin strict CommonMark both sides and test renderer parity (same md in, diff rendered DOM) or hydration flashes divergent rendering.

## 2026-08-12 — build start, decisions locked

Name: vitrine. Layout: .vitrine/<slug>/ (fixed ../../ depth to .tisket/.zettel). Serving: per-repo vitrine serve (daemon later). Extraction in vitrine via comrak AST (mdstore has no AST; migrate only if it grows one). Artifact = self-contained dir (index.html + vitrine-runtime.js); file:// tier renders from baked DSD without JS. Render profile: plain CommonMark both sides (no header-id ext in output — anchors computed internally) so comrak/commonmark.js parity is achievable and bake vs hydrate DOMs match. CLI: new, sync, serve, resolve, extract, render, docs. Response inbox: POST /respond/<slug> → responses/ + latest.json; deterministic-stamp env knob for tests.

## 2026-08-12 — vitrine v0.1.0 shipped

Built and shipped in full: extract (fence-aware ATX slugging, dupe suffixes), refs (scheme→relative-path at authoring time, ambiguity errors), bake (light-DOM fill, idempotent, specific errors), serve (static + inbox, traversal-hardened, watcher stdout line, VITRINE_STAMP determinism knob), scaffold (self-contained dir: template + vendored commonmark.js 0.31.2 + component runtime), CLI (new/sync/serve/resolve/extract/render/docs), bundled docs.

Tests: 15 cargo + 5 missouri paths, zero clippy warnings. The renderer-parity property is a missouri path: comrak and commonmark.js byte-identical on the fixture. Two test bugs found and fixed during green-up: shell quoting collapsed the node parity helper; curl silently normalized the /respond/../etc traversal probe (needs --path-as-is), meaning the first 'passing' traversal check had never reached the server.

Shipped: github.com/cjohnhanson/vitrine (public), nix flake, co.d input + hms — vitrine on PATH. Remaining for the vision: tisket/zettel integration conveniences (artifact attach/show), daemon-with-registry, explicit block anchors. This repo's first vitrine artifact is the natural next dogfood.

## Form-control styling (post-ship polish)

Native controls are fully styleable because everything renders in light DOM. Template now ships: appearance:none select with inline-SVG chevron, accent-colored button with hover/focus-visible rings, flex form layout — all on the existing CSS variables so light/dark holds. Scaffolded fixture regenerated, 5/5 missouri green, pushed, co.d pin bumped.
