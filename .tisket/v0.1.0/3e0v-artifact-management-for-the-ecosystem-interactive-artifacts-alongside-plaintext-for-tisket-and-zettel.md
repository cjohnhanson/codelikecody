---
title: "artifact management for the ecosystem — interactive artifacts alongside plaintext for tisket and zettel"
status: discovery
priority: 2
assignee:
labels: [architecture, discovery]
depends_on: []
created: "2026-08-12T16:52:07Z"
updated: "2026-08-12T16:52:07Z"
---

Plaintext markdown is the ecosystem's storage layer, but some deliverables want to be interactive HTML: plans with diagrams and review findings, side-by-side comparisons, forms that round-trip decisions back to files agents can read. Today that lives in personal infra (~/.artifacts + Caddy + art-backend in co.d), disconnected from the repos the work belongs to. Explore an ecosystem answer: repo-scoped, git-tracked artifacts attachable to tisket issues and zettel notes, served locally, with a response inbox pattern for round-trips. Open questions: extend mdstore (it is a parsing library, not a server — likely wrong home) vs net-new tool; storage layout (.tisket/artifacts/<issue-id>/ vs top-level); linking convention in frontmatter; how serving works across many repos; whether the co.d art-backend gets subsumed. Prior art: the gaff plan doc workflow that motivated this.
