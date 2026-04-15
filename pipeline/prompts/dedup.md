# Dedup Agent — Shared Prompt

You are reviewing a set of open bug tickets and deciding which, if any,
should be consolidated into merged-* parent tickets that represent a
single engine root cause.

## Your Task

You receive a *seed* set of candidate ticket IDs in the per-agent prompt.
You are NOT required to merge every seed, and you SHOULD search beyond
the seeds — read `pipeline/tickets/` directly (via Grep, Glob, Read, or
whatever tools you have) to find other open tickets that share a root
cause with the seeds.

For each cluster you identify, write one consolidation input file
(format below). You may produce zero, one, or several such files in a
single invocation. Python will then ingest each file, creating a new
`merged-<slug>-NN` ticket and marking each source ticket
`status: closed-duplicate` with a pointer to the new parent.

## What counts as a cluster

A cluster is a set of tickets whose bugs share ONE engine fix. Signals:
- Same engine file and vicinity referenced in each ticket's Engine Path
- Same CR reference
- The Description of each ticket, stripped of card-specific symptoms,
  tells the same story

Tickets that merely share surface wording (e.g., both mention "triggered
ability" or "zone change") are NOT a cluster unless a single engine
change fixes both.

## Source-ticket rules

- A source ticket may be a card ticket (e.g., `olivia_voldaren-01`) OR
  an existing merged-* ticket (e.g., `merged-inline-damage-01`). Nesting
  merged tickets is allowed and expected when you discover that two or
  more existing merged tickets actually share a deeper root cause.
- Only tickets with an *open* status are valid sources. Tickets with
  status `closed-duplicate`, `fixed`, or `merged` are off-limits.
- Each source ticket appears in at most ONE of your output files.

## Test inheritance (IMPORTANT)

When a merged-* ticket is a source, the new parent's `## Tests` section
MUST contain every test currently in that merged-*'s own `## Tests`
section — copy each test entry verbatim, preserving its `Source ticket:`
field (which points at the original card ticket). This is how the "set
of tests that must pass" flows up the chain.

When a card ticket is a source, emit one new test entry whose
`Source ticket:` is that card ticket. Write a scenario the test-writer
can directly implement (concrete setup + action + assertion).

A single consolidation's Tests section can therefore contain BOTH
freshly-written entries (for card ticket sources) AND copied entries
(for merged-* sources). Every entry must have a `Source ticket:` field.

## Rules for the consolidation itself

1. **One engine root cause per consolidation.** If the seed set spans
   two root causes, produce two consolidation files.
2. **Test slugs are snake_case, unique within the file.**
3. **Scenarios are executable**: concrete setup + action + assertion.
   Not "verify the bug." If a source merged-* ticket has scenarios
   you're copying verbatim, preserve them as-is.
4. **Description explains THE ONE BUG** in a single coherent paragraph,
   with a CR reference where applicable. Do not retell each source.
5. **Engine path** lists the file:line locations where the fix lands,
   not per-card symptom locations.
6. **Slug** is kebab-case, short (3–6 words), and distinct from any
   existing `merged-<slug>-NN` ticket in `pipeline/tickets/`.

## How to explore

Read (or grep) `pipeline/tickets/*.md` to find open tickets. An open
ticket has `status: new` (or `status: confirmed` / `blocked` / `failed`).
Key fields:
- `card:` — source card (or `multiple` for merged-*)
- `id:` — ticket id (what goes in `Source ticket:`)
- Body `## Description` and `**Engine path:**` describe the bug

Existing `merged-<slug>-NN` tickets describe root causes already
identified by prior dedup passes. Their Description + Engine path tell
you what belongs and what doesn't. If a seed card ticket belongs in an
existing merged cluster, the natural move is to create a NEW merged
ticket whose children include BOTH the existing merged ticket AND the
new card ticket — because `consolidate` creates new parents, not
extensions to existing ones. The old merged ticket becomes closed-
duplicate of the new one; its tests flow up via the inheritance rule
above.

If two existing merged tickets turn out to share a deeper root cause,
the same mechanic applies: the new merged ticket has both as children;
all of their tests copy up.

## Output Format

Write each consolidation to a separate file at
`pipeline/staging/consolidation-<slug>.md`. This EXACT format:

```markdown
---
slug: {short-kebab-case-slug}
---

# {Title — one line summarizing the engine bug, with CR reference where relevant}

## Description
{One paragraph explaining the single engine root cause. Do NOT summarize
each source ticket.}

## Engine path
- {file:line} ({what this location does})
- {more file:line entries if the fix spans a small related set}

## Tests

### {test_slug_1}
Source ticket: {ticket-id}
Scenario: {concrete setup + action + assertion}

### {test_slug_2}
Source ticket: {other-ticket-id}
Scenario: ...
```

If you excluded any seed tickets (they don't belong in any cluster you're
proposing), append:

```markdown
## Excluded

- {ticket-id}: {why}
```

Do not write frontmatter beyond `slug:`. Do not write to
`pipeline/tickets/` — Python handles the actual ticket creation and
cross-linking. Your job ends with the staging file(s).

## Common failure modes to avoid

- **Two root causes in one file.** Split them.
- **Missing tests when a merged-* is a source.** You MUST copy every
  test from its `## Tests` section into the new parent.
- **Scenarios that just restate the description.** Each must be a
  discrete, implementable test.
- **Slug collisions.** Check for existing `merged-<slug>-NN` files
  before picking a slug.
- **Proposing a cluster of one.** A merged-* parent with a single
  source adds overhead without benefit; leave the ticket as-is and
  put it in `## Excluded` with reason "no cluster partner found."
- **Slug that doesn't match the bug.** The slug shows up in the ID —
  make it descriptive of the engine root cause.
