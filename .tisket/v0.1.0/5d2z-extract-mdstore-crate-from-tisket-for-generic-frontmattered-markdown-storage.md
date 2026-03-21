---
title: "Extract mdstore crate from tisket for generic frontmattered markdown storage"
status: in_progress
priority:
assignee:
labels: [refactor]
depends_on: []
created: 2026-03-21T19:31:14Z
updated: "2026-03-21T19:32:26Z"
---

Extract the domain-agnostic parts of tisket into a shared mdstore crate that both tisket and future tools (zettel) can depend on.

## What moves to mdstore

- Frontmatter parse/serialize — generic over T: Serialize + DeserializeOwned instead of hardcoded IssueFrontmatter
- Prefix+slug ID system (slug.rs: slugify, generate_prefix, extract_prefix, has_prefix)
- Directory scanning — walk a dir for .md files, parse each into Document<T>
- Git context (git.rs: GitContext, BranchStatus, branch-aware divergence detection)
- Config pattern — a root config file pointing to a data directory

## What stays in tisket

- IssueFrontmatter, Status enum, issue-specific logic
- Repo methods that are issue-workflow-specific (close/reopen with status transitions, scratch notes)
- CLI (cli.rs)
- Selector system

## Safety

~40 missouri test states cover the full tisket CLI surface. The refactor is purely internal — if the CLI produces the same filesystem output, tests pass.

## Approach

1. Create mdstore crate in workspace
2. Move generic code, parameterize frontmatter parsing over T
3. Make tisket depend on mdstore
4. Missouri tests green throughout
5. zettel can then depend on mdstore directly

## Scratch Notes
