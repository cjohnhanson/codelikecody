<!-- metadata
title: "zettel CLI Reference"
description: "Complete command reference for the zettel knowledge base"
type: reference
-->

# zettel CLI Reference

zettel is a zettelkasten-style knowledge base built on frontmattered markdown. Notes are stored as individual markdown files with YAML frontmatter in a `.zettel/` directory, using the same prefix+slug ID system as tisket (via the shared mdstore library).

## Global Options

`--root <path>` — Root directory of the repository. Defaults to `.` (current directory). Applies to all subcommands.

`--version` — Print version and exit.

`--help` — Print help and exit.

## Commands

### `zettel init`

Initialize zettel in the current directory. Creates `zettel.yml` at the root and a `.zettel/` directory.

Fails if `zettel.yml` already exists.

### `zettel backlinks <id>`

Show all notes that link to the given note. Checks both frontmatter `links:` fields and `[[id]]` references in note bodies. Links are resolved flexibly — a slug-only reference matches its prefixed note.

| Option | Default | Description |
|--------|---------|-------------|
| `--format <fmt>` | `text` | Output format: `text` or `json` |

Text output columns: `ID`, `TITLE`.

JSON output is an array of `{ "id": "...", "title": "..." }` objects.

### `zettel orphans`

Show notes with no links — neither incoming nor outgoing. A note is an orphan if:
- Its `links:` field is empty
- No other note's `links:` field references it
- No other note's body contains a `[[id]]` reference to it
- Its own body contains no `[[id]]` references to other notes

---

## `zettel note` Subcommands

### `zettel note create <title>`

Create a new note. The title is slugified to produce the filename (e.g., "Attention is not explanation" becomes `ab12-attention-is-not-explanation.md`, where `ab12` is a randomly generated 4-character `[a-z0-9]` prefix). Duplicate slugs are rejected.

Prints the generated note ID to stdout.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tags <csv>` | `-t` | | Comma-separated tags |
| `--links <csv>` | `-l` | | Comma-separated link IDs (other notes this one references) |
| `--body <text>` | `-b` | | Note body text, inline |

### `zettel note list`

List all notes. Optionally filter by tag.

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--tag <tag>` | `-t` | | Filter to notes with this tag |
| `--format <fmt>` | | `text` | Output format: `text` or `json` |

Text output: `ID  [tags]  TITLE`.

JSON output is an array of note objects (see JSON output format below).

### `zettel note show <id>`

Show full details for a note.

| Option | Default | Description |
|--------|---------|-------------|
| `--format <fmt>` | `text` | Output format: `text` or `json` |
| `--field <name>` | | Extract a single field value. Valid fields: `title`, `tags`, `links`, `body`, `id` |

When `--field` is specified, only that field's value is printed. For list fields (`tags`, `links`), values are comma-separated.

### `zettel note edit <id>`

Edit an existing note's metadata or body. Only specified options are changed; everything else is preserved.

| Option | Description |
|--------|-------------|
| `--title <text>` | Replace the title |
| `--tags <csv>` | Replace all tags (comma-separated) |
| `--add-tag <tag>` | Add a single tag, keeping existing ones |
| `--remove-tag <tag>` | Remove a single tag, keeping others |
| `--links <csv>` | Replace all links (comma-separated) |
| `--add-link <id>` | Add a link to another note |
| `--remove-link <id>` | Remove a link |
| `--body <text>` | Replace the entire body |
| `--append <text>` | Append text to the body |

Updates the `updated` timestamp automatically.

### `zettel note delete <id>`

Permanently delete a note. Removes the markdown file from `.zettel/`.

---

## ID Resolution

Note IDs are resolved flexibly. Any of these forms work wherever `<id>` is accepted:

- **Full ID** — `ab12-attention-is-not-explanation` (exact filename stem match)
- **Short prefix** — `ab12` (4-character `[a-z0-9]` prefix, must be unambiguous)
- **Slug portion** — `attention-is-not-explanation` (the part after the prefix, must be unambiguous)

Ambiguous prefix matches produce an error.

---

## Link Syntax

Notes can reference each other in two ways:

1. **Frontmatter links** — the `links:` field in YAML frontmatter lists note IDs this note explicitly connects to.
2. **Inline references** — `[[id]]` syntax anywhere in the note body. The ID inside brackets follows the same resolution rules (full ID, prefix, or slug).

Both forms are recognized by `zettel backlinks` and `zettel orphans`.

---

## File Format

### Repository Configuration: `zettel.yml`

Lives at the repository root.

```yaml
zettel_dir: .zettel
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `zettel_dir` | string | `.zettel` | Directory where notes are stored |

### Note Files

Notes are markdown files at `<zettel_dir>/<id>.md`.

The filename stem is the note ID: a 4-character random prefix, a hyphen, and a slugified title (e.g., `ab12-attention-is-not-explanation.md`).

#### Structure

```
---
<YAML frontmatter>
---

<body — free-form markdown>
```

#### Frontmatter Schema

```yaml
title: "Attention is not explanation"
tags: [ml, interpretability]
links: [cd34-transformer-architecture]
created: "2026-03-21T10:30:00Z"
updated: "2026-03-21T10:30:00Z"
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `title` | string | yes | | Note title. Quoted in serialization |
| `tags` | list of strings | no | `[]` | Freeform tags for categorization |
| `links` | list of strings | no | `[]` | IDs of notes this one references |
| `created` | string or null | no | auto | ISO 8601 timestamp, set at creation |
| `updated` | string or null | no | auto | ISO 8601 timestamp, updated on every edit |

---

## JSON Output Format

When `--format json` is used, each note is represented as:

```json
{
  "id": "ab12-attention-is-not-explanation",
  "title": "Attention is not explanation",
  "tags": ["ml", "interpretability"],
  "links": ["cd34-transformer-architecture"],
  "body": "The attention mechanism in transformers does not..."
}
```

`zettel note list --format json` returns an array of these objects.

---

## Directory Layout

```
repo/
  zettel.yml
  .zettel/
    ab12-attention-is-not-explanation.md
    cd34-transformer-architecture.md
    ef56-gradient-descent-intuition.md
```
