---
name: writing-review
description: >
  Systematic writing evaluation using Orwell's rules, Williams' clarity
  diagnostics, readability metrics, and context-appropriate frameworks.
  Adapts to content type: technical docs get Google/Plain Language checks,
  persuasive copy gets AIDA/PAS, explanations get Williams' old-new
  principle. Dispatches sub-agents as independent evaluators. Use when
  reviewing writing quality, when the user invokes /writing-review, or
  before shipping any user-facing text. Not for code review or design
  review.
user-invocable: true
---

# Writing Review

Evaluate writing quality using frameworks matched to the content's
purpose. Sub-agents evaluate independently; the orchestrator synthesizes.

## Phase 1 — Classify the content

Before applying any framework, determine what kind of writing this is:

| Type | Signals | Primary frameworks |
|---|---|---|
| **Tutorial** | Step-by-step, imperative verbs, code blocks | Plain Language, Google Tech Writing, readability |
| **Reference** | Tables, exhaustive listings, factual | Google Tech Writing, consistency checks |
| **Explanation** | Why/how, conceptual, narrative | Williams (clarity), Orwell, old-new principle |
| **How-to guide** | Task-oriented, assumes competence | Plain Language, AIDA (problem → solution) |
| **Landing/marketing** | Persuasive, conversion goal | AIDA, PAS, voice consistency |
| **README** | First impression, mixed purposes | All of the above in miniature |
| **Essay/opinion** | Argument, voice, perspective | Orwell, Williams, voice guide if one exists |

Read the content. Identify its type. This determines which frameworks
apply with what weight.

## Phase 2 — Sentence-level evaluation (Orwell + Williams)

Apply to all content types. These are universal.

**Orwell's 6 Rules** — check each sentence:
1. No dead metaphors or clichés
2. Short words over long ones
3. Cut every word that can be cut
4. Active voice over passive
5. Everyday words over jargon
6. Break any rule to avoid sounding barbarous

**Williams' diagnostic** — for each paragraph:
1. Underline the first 7-8 words of each sentence
2. Do they name the main character and state the main action?
3. Are actions expressed as verbs, not nominalizations (-tion, -ment, -ness)?
4. Does each sentence begin with familiar information and end with new?

Report specific violations with line/paragraph references and
suggested rewrites.

## Phase 3 — Structural evaluation

Framework depends on content type.

**For tutorials and how-to guides (Plain Language):**
- Most important information first?
- Conditions before actions ("If X, do Y" not "Do Y if X")?
- One topic per paragraph?
- Lists instead of dense paragraphs for 3+ items?
- Average sentence length under 20 words?
- "You" and "we" instead of third person?

**For explanations (Williams + old-new):**
- Does each paragraph have a clear topic sentence?
- Does information flow from known to unknown?
- Are complex ideas built up through accumulation?
- Are qualifications and exceptions placed after the main point?

**For persuasive content (AIDA/PAS):**
- **AIDA**: Does it capture Attention → build Interest → create Desire → prompt Action?
- **PAS**: Does it name the Problem → Agitate → present Solution?
- Is every claim supported with evidence or example?
- Is the call to action clear and specific?

**For reference docs:**
- Is terminology used consistently throughout?
- Are all terms defined at first use?
- Is the structure parallel (same pattern for each entry)?
- Can a reader find specific information without reading everything?

## Phase 4 — Readability metrics

Run on all content. Report the numbers with context.

- **Flesch-Kincaid Grade Level** — target depends on audience:
  - Consumer content: grade 6-8
  - Developer docs: grade 8-12
  - Academic/specialist: grade 12+
- **Average sentence length** — flag paragraphs over 25 words average
- **Paragraph length** — flag paragraphs over 6 sentences

These are signals, not verdicts. A grade-14 passage might be fine
if the audience is specialists. A grade-6 passage might be wrong if
it oversimplifies technical content.

## Phase 5 — Voice consistency

If a voice guide exists (check for `references/voice-guide.md` or
similar), evaluate against it.

If no voice guide exists, check for internal consistency:
- Does the register stay consistent? (Don't mix formal and casual)
- Is the point of view consistent? (Don't switch between "you" and "the user")
- Is terminology consistent? (Don't call it "server" then "machine" then "instance")
- **The Mailchimp test**: could this have been written by a different
  brand/author? If yes, the voice is weak.

## Phase 6 — Fresh eyes

Spawn a sub-agent with NO context from phases 1-5. It reads the
content cold and reports:

1. After the first paragraph, do you want to keep reading?
2. What's the single main point? Can you state it in one sentence?
3. Where did you get confused or have to re-read?
4. What's missing that you expected to find?
5. Quote the weakest sentence and say why.
6. Quote the strongest sentence and say why.

## Phase 7 — Report and fix

For each finding:
- Which framework flagged it
- Severity (blocks shipping / degrades quality / cosmetic)
- The specific text
- A rewrite

Fix everything. Recheck affected sections. Loop until clean.
