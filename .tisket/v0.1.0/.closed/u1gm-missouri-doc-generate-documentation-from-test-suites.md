---
title: "missouri doc: generate documentation from test suites"
status: done
priority: 2
assignee: coord-docs
labels: [missouri, docs]
depends_on: []
created: 2026-03-22T13:56:52Z
updated: "2026-03-22T14:13:31Z"
---

Generate documentation from missouri test suites. Tests are the source
of truth; docs are a rendered view over them.

## The idea

Missouri tests already contain everything a tutorial needs: shell commands
(transitions), expected output (stdout assertions), complete file trees
(state directories), and the sequence showing how things evolve. The missing
piece is prose — connective tissue between steps.

Add a `doc` field to states and transitions in missouri.yml:

```yaml
doc: |
  Initialize tisket in the repository.

transitions:
  - name: "tisket init"
    command: "tisket init"
    doc: |
      The generated tisket.yml configures where issues are stored.
    target: "../initialized"
```

`missouri doc` renders a test path as a document, interleaving:
1. State prose
2. File tree (excluding ignored files)
3. Transition prose
4. Command in a console block
5. Expected stdout
6. Next state file tree (or diff showing what changed)

## Output formats

- Markdown (for static docs, piping to other tools)
- JSON (for consumption by docs-web or other renderers)

## Integration with docs-web

The docs-web Leptos app can render missouri doc output as interactive
pages with:
- Accordion file explorers at each state
- Syntax-highlighted file content
- Before/after diffs between states
- The actual command and output
- Embedded asciicast recordings (missouri already has --record)

## What this replaces

Hand-written tutorials that describe commands and output in prose.
Instead, the tutorials ARE the tests — verified on every run, impossible
to drift from reality.

## Done when

- `doc` field parsed from missouri.yml (states and transitions)
- `missouri doc` renders a test path as markdown
- File trees rendered with ignore patterns applied
- At least one existing test suite (tisket or clc) annotated with doc fields
- Rendered output is usable as a standalone tutorial

## Scratch Notes

### Approach
Add `doc: Option<String>` to StateConfig, TransitionConfig, State, Transition.
New `docgen.rs` module with `render_markdown` and `render_json` functions.
New `Doc` subcommand in cli.rs.

### Key files
- `missouri/src/config.rs` — add doc fields to StateConfig + TransitionConfig
- `missouri/src/graph.rs` — add doc fields to State + Transition; propagate from config
- `missouri/src/docgen.rs` — NEW: rendering logic (stubs + tests)
- `missouri/src/cli.rs` — add Doc subcommand, dispatch to docgen
- `missouri/src/lib.rs` — register docgen module

### Docgen API
- `render_markdown(graph: &StateGraph, path: &TestPath) -> String`
- `render_json(graph: &StateGraph, path: &TestPath) -> serde_json::Value`

### Phase
Currently: tests-unwritten → writing tests + minimal stubs
