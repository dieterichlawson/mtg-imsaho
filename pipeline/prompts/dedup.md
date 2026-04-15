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

**Invariant (enforced by Python):** the new parent's `## Tests` section
must contain every test (identified by `### {slug}`) that exists in the
`## Tests` section of any ticket being closed. No test may be silently
dropped during consolidation. Python will refuse to ingest your output
if any closed ticket has a test slug that doesn't appear in the parent.

Practically this means:

- When a card ticket has 1+ tests, copy EVERY one into the new parent.
  Preserve the slug verbatim; set `Source ticket:` to the card ticket's
  id (even if the original read `Source ticket: (new)`). A card ticket
  with three tests contributes three entries to the parent, all with
  the same card-ticket id as `Source ticket:`.
- When a merged-* ticket is being closed (via `## Also closes`), copy
  EVERY test from its `## Tests` section into the new parent verbatim,
  preserving both the slug and the original `Source ticket:` field
  (which already points at a card ticket).
- You may add new tests that weren't in any closed ticket — for card
  tickets in the seed set that haven't been merged before, write a
  fresh entry.

A single consolidation's Tests section can therefore contain BOTH
freshly-written entries AND copied entries. Every entry must have a
`Source ticket:` field pointing at a card ticket. The set of slugs must
cover the union of all descendant tests.

## Closing merged-* tickets whose tests you absorbed (IMPORTANT)

When you copy tests out of an existing merged-* ticket into your new
parent, that merged-* ticket itself must also be closed — otherwise
it's left as an orphan pointing at children who now live elsewhere. The
card-ticket `Source ticket:` entries on your copied tests do NOT close
the intermediate merged-* ticket; they only close the card tickets.

Use the `## Also closes` section for this. Every merged-* ticket whose
tests you copied into your new parent must appear in `## Also closes`.
Python will then mark them `status: closed-duplicate` with
`duplicate_of:` pointing at the new parent — without requiring a
synthetic test entry.

```markdown
## Also closes

- merged-fake-widget-target-check-01
- merged-fake-widget-cleanup-check-01
```

Rules:
- Every merged-* ticket you treat as a source (whose tests you copied)
  MUST appear in `## Also closes`.
- A ticket cannot appear in both `Source ticket:` (per-test) and
  `## Also closes`. Pick one.
- `## Also closes` is optional — omit the section when you're not
  nesting any merged-* tickets.

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
