---
name: documentation-writing
description: >
  Guides collaborative writing of project documentation through nested
  review loops. Individual docs go through draft → fresh-eyes → fix
  cycles. The full doc set goes through a site-level coherence review.
  Applies Diátaxis type discipline, cross-linking, source verification,
  and prose quality checks. Triggered when asked to write, draft, or
  create docs, guides, tutorials, references, or explanations. Not
  triggered for inline code comments, commit messages, PR descriptions,
  or README updates.
---

# Documentation Writing

## The core problem this skill solves

Documentation fails in two ways. The obvious way: it gets written in one
shot and presented as finished — mechanically correct but lifeless. The
subtler way: individual docs pass review but the doc set as a whole
doesn't cohere — concepts are mentioned but not linked, pages assume
knowledge that's explained elsewhere, and there's no natural reading
path through the site.

This skill addresses both through nested review loops.

## Two loops

### Inner loop — individual doc

Every doc goes through this cycle. Don't move to the next doc until the
current one passes.

1. **Context** — determine doc type, audience, purpose, scope
2. **Draft** — write the doc (section-by-section if interactive, full
   draft if batching)
3. **Fresh-eyes review** — spawn a sub-agent that has NOT seen the
   drafting conversation. It reads the doc cold and evaluates:
   - Does the opening earn attention? Would you keep reading?
   - Is every paragraph pulling its weight, or is there filler?
   - Are concepts mentioned that aren't explained or linked?
   - Does the voice sound human or generated?
   - What's the weakest section? Quote it.
4. **Fix** — address the review findings
5. **Repeat** steps 3-4 until the fresh-eyes pass comes back clean

The fresh-eyes review is the quality gate. Accuracy checks (source
verification, type discipline) are necessary but not sufficient. The
fresh-eyes review checks whether the doc is *good*, not just *correct*.

### Outer loop — whole site

After all individual docs have passed their inner loops:

1. **Site-level fresh-eyes review** — spawn a sub-agent that navigates
   the full doc set as a reader would. It:
   - Lands on the index page and tries to find something specific
   - Follows cross-links between docs
   - Reads two or three docs end-to-end
   - Evaluates: is there a natural reading path? Are there dead ends?
     Do concepts get introduced without links? Is the voice consistent
     across docs? Does the sidebar ordering make sense?
2. **Cross-linking pass** — find every concept mention that's explained
   in another doc and add a link on first mention. Don't over-link —
   link a term once per doc, on first use.
3. **Fix** — address site-level issues (may require going back into
   inner loops for specific docs)
4. **Repeat** until the site-level pass comes back clean

### Separate evaluator prompts

Use different sub-agents for different review dimensions. Don't ask one
agent to check everything — accuracy reviewers check different things
than quality reviewers.

**Accuracy evaluator** (for reference docs especially):
- Is every CLI flag, config field, and default value correct?
- Spot-check 5-10 claims against source code
- Are there hallucinated features or behaviors?

**Quality evaluator** (for all docs):
- Does the opening paragraph hook the reader?
- Is every paragraph earning its place?
- Are there sentences that sound generated rather than written?
- Quote the weakest passage and say why it's weak
- What's missing that a reader would want to know?

**Site coherence evaluator** (outer loop):
- Navigate the docs as a new user would
- Follow 3-5 cross-links — do they work? Do they land in the right place?
- Are the same concepts explained differently in different docs?
- Is there a natural "what to read first → what to read next" flow?
- What's the biggest gap in the doc set?

## Cross-linking

Docs reference concepts explained elsewhere. Link them.

**Rules:**
- Link a concept on first mention in each doc, not every mention
- Use the route slug as the link target (e.g., `/clc/phase-system`)
- Don't link obvious terms that any developer would know
- Do link project-specific terms: phases, worktrees, tiskets, scratch
  notes, guard, prime text, coordinators, workers, divergence detection
- When mentioning a tool feature in passing, link to its doc rather than
  re-explaining inline

**Cross-linking pass:** after all docs are drafted, read each doc
looking for unlinked concept mentions. Add links. This is a distinct
step, not something done during drafting — it's easier to catch gaps
when reading the finished text.

## File conventions

Documentation lives in per-crate `docs/` directories:
- `clc/docs/` — ecosystem-level docs and clc-specific docs
- `missouri/docs/` — missouri-specific docs
- `tisket/docs/` — tisket-specific docs

Every doc file uses an HTML comment metadata block (compatible with
both mdBook and the docs-web Leptos app):

```html
<!-- metadata
title: "The document title"
description: "One-sentence summary for indexes and search"
type: tutorial | guide | reference | explanation
-->
```

File naming: lowercase, hyphens, descriptive.

Internal links use absolute route slugs without `.md` extensions:
`[phase system](/clc/phase-system)`.

## Diátaxis coverage

Each tool should have docs in all four quadrants:

| | Tutorial | Guide | Reference | Explanation |
|---|---|---|---|---|
| Per tool | Getting started | How-to guides | CLI reference | What is / why |

Not every quadrant needs to be filled immediately, but gaps should be
identified and tracked.

## Type discipline

One document, one Diátaxis type. The failure modes:

- **Reference in tutorials.** Tree diagrams, exhaustive field listings,
  full API surfaces. If it describes *what exists* rather than *what to
  do next*, extract it and link to the reference.

- **Reference in explanations.** Phase tables, flag listings. The
  explanation discusses *why*; link to reference for *what*.

- **How-to steps in reference.** Workflow narratives in a reference doc.
  That's a guide. Split it.

## Reference doc verification

Every technical claim in a reference doc must be verified against source
code, CLI `--help` output, or test fixtures. After drafting, do a
verification pass: grep/read the codebase for every field, flag, and
behavior mentioned. Mark unverifiable claims with
`<!-- unverified: [reason] -->`.

The default assumption: any technical detail written from memory is
wrong until confirmed against source.

## Voice and tone

Write for a competent peer. Direct, technically precise, conversational
but not sloppy.

**Avoid:** corporate blog openings ("This guide walks you through..."),
over-explanation, forced enthusiasm, inspirational framing, hedging
without reason. See [references/voice-examples.md](references/voice-examples.md).

**Do:** state things plainly, say what's true including limitations,
use real runnable code examples, lead with content not meta-commentary.

## When this skill applies

- New docs from scratch
- Major rewrites of existing docs
- Splitting a mixed-type doc into proper single-type docs
- Site-level coherence reviews across the doc set

Does not apply to:
- Quick edits to fix a typo or update a version
- Inline code comments or docstrings
- README files (unless treated as proper docs)
- PR descriptions, commit messages, or issue write-ups

## Checklist — individual doc

- [ ] Metadata includes title, description, and type
- [ ] File is in the correct crate's `docs/`, named with lowercase hyphens
- [ ] Document serves exactly one Diátaxis type
- [ ] Every technical claim in reference docs is verified against source
- [ ] Concepts mentioned from other docs are linked on first use
- [ ] Opening paragraph earns the reader's attention
- [ ] No filler paragraphs or generated-sounding prose
- [ ] Fresh-eyes sub-agent review passed clean

## Checklist — site level

- [ ] Every tool has docs in all four Diátaxis quadrants (or gaps tracked)
- [ ] Cross-links work and land on the right pages
- [ ] Voice is consistent across docs
- [ ] A new reader can navigate from index to any topic without dead ends
- [ ] Same concepts aren't explained differently in different docs
- [ ] Site-level fresh-eyes review passed clean
