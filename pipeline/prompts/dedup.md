# Dedup Agent — Shared Prompt

You are reviewing a set of open bug tickets and deciding which, if any,
should be consolidated into merged-* parent tickets that represent a
single engine root cause.

## Your Task

You receive an exact set of candidate ticket IDs in the per-agent prompt.
Consider merging only those tickets. **Do NOT search `pipeline/tickets/`
for other tickets, and do NOT propose a cluster that pulls in tickets
outside the given set.** If you notice a related open ticket that was
not passed in, say so in free-form text at the end of your response —
but do not merge it. The human deliberately selected this group; your
job is to decide which subsets are worth merging.

A ticket passed in may be either a **card ticket** (e.g.
`olivia_voldaren-02.md`) or an existing **`merged-*` ticket**. Both are
valid cluster members, and the latter may be nested under a deeper
parent via `## Also closes`.

For each cluster you identify, write one consolidation input file
(format below). You may produce zero, one, or several such files in a
single invocation. Python will then ingest each file, creating a new
`merged-<slug>-NN` ticket and marking each source ticket
`status: closed` with `closed_reason: absorbed` and
`absorbed_into: <new-parent-id>`.

## What counts as a cluster

A cluster is a set of **currently-open tickets** that can be closed by
**one atomic fix** — one commit, one set of test updates, one merge.
Signals:
- Same engine file and vicinity referenced in each ticket's Engine Path
- Same CR reference
- The Description of each ticket, stripped of card-specific symptoms,
  tells the same story
- You can concretely state a single code change that closes every
  ticket in the proposed cluster

Two things that LOOK like duplicates but aren't:

1. **Tickets that merely share surface wording** (e.g., both mention
   "triggered ability" or "zone change") are NOT a cluster unless a
   single code change fixes both.

2. **A new ticket matching the pattern of an already-shipped fix is
   NOT a duplicate.** If card X has an inline-damage bug exactly like
   the one `merged-inline-damage-02` shipped a fix for, but card X
   wasn't in that fix's scope, card X's ticket still needs its own
   fix. The shipped fix is gone from your view (its parent is
   archived) and cannot be extended retroactively. Leave the new
   ticket standalone — the subsequent `test → fix → merge` flow will
   handle it as a one-off. Do NOT try to close it as a duplicate.

The principle: **duplication is about shared FIX, not shared PATTERN.**
Tickets fixed by the same commit = cluster. Tickets that happened to
exhibit the same underlying pattern but require separate edits =
separate work.

## Source-ticket rules

- A source ticket may be a card ticket (e.g., `olivia_voldaren-01`) OR
  an existing merged-* ticket (e.g., `merged-inline-damage-01`). Nesting
  merged tickets is allowed and expected when you discover that two or
  more existing merged tickets actually share a deeper root cause.
- The tickets in `pipeline/tickets/*.md` are the ONLY ones you can
  propose to absorb. Terminal tickets are moved to
  `pipeline/tickets/archive/` and are off-limits.
- **Only tickets in `status: new` or `status: tested` may be absorbed.**
  `fixed` and `fix_failed` carry downstream state (fix commits, a
  post-mortem) that consolidation would silently drop — they are
  rejected outright by Python.
- **At most one absorbed source may be `tested`.** If multiple
  candidates are `tested`, pick one to absorb and leave the others
  as-is (the human will decide how to reconcile). Python rejects
  proposals with >1 `tested` source.
- When you absorb a `tested` source, preserve its test slugs
  **verbatim** on copied tests — Python uses slug matching to carry
  the existing `Implementation:` pointer forward into the new parent.
  If you rename a slug, Python treats it as a fresh entry needing
  re-implementation.
- `also_closes` entries MUST be in `new` or `tested`. Python rejects
  closed/fixed/fix_failed ids here.
- Per-test `source_ticket` values may reference ANY ticket id (open or
  already-closed). When you copy a test verbatim from a merged-*
  source, keep its original `source_ticket` pointer intact — Python
  uses it for coverage counting and silently treats already-closed
  pointers as metadata (it does NOT try to re-absorb them). Open
  source_ticket values ARE absorbed alongside also_closes.
- Each ticket id appears in at most ONE of your output files.

## Test inheritance (IMPORTANT)

**Invariant (enforced by Python):** for every ticket being closed, the
parent must contain at least as many tests attributable to each of its
Source tickets as the closed ticket did. "Attributable to X" means the
test has `Source ticket: X` in the parent. Python will refuse to ingest
if any closed ticket loses test coverage.

This is a COUNT-based check (not slug-based) — slugs may be specialized
when copying up. What matters is the per-Source-ticket total.

Rules for composing the Tests section:

- **Card ticket sources:** when a card ticket is a source (the agent's
  reference to it appears in a `Source ticket:` field), the parent
  must have at least as many tests carrying that card's id as the
  card had tests. Audit-generated card tickets typically have their
  own tests with `Source ticket: (new)`; those count against you once
  you adopt them — you must pull across ALL of them, re-pointing the
  `Source ticket:` to the card's id.
- **Merged-* sources (via `## Also closes`):** a merged-* ticket's
  tests already carry `Source ticket:` pointing at its descendant
  card tickets. Copy each test verbatim (preserving its `Source
  ticket:` field) into the parent — so the parent's counts include
  every per-Source total the merged-* had.
- **Fresh additions:** you may add new tests for seed card tickets
  that weren't previously merged. These count toward that seed's
  required total.

A single consolidation can therefore contain copied entries AND freshly-
written entries. Slugs may differ from the originals if you prefer
(e.g., specializing `activated_ability_goes_on_stack` to
`full_moons_rise_activated_ability_on_stack`), but the Source-ticket
counts must meet every closed ticket's requirement.

## Closing merged-* tickets whose tests you absorbed (IMPORTANT)

When you copy tests out of an existing merged-* ticket into your new
parent, that merged-* ticket itself must also be closed — otherwise
it's left as an orphan pointing at children who now live elsewhere. The
card-ticket `Source ticket:` entries on your copied tests do NOT close
the intermediate merged-* ticket; they only close the card tickets.

Use the `## Also closes` section for this. Every merged-* ticket whose
tests you copied into your new parent must appear in `## Also closes`.
Python will then mark them `status: closed` with
`closed_reason: absorbed` and `absorbed_into:` pointing at the new
parent — without requiring a synthetic test entry.

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

Read (or grep) `pipeline/tickets/*.md` to find open tickets. That
directory contains ONLY open tickets — Python moves terminal ones
(`status: closed`, `status: shipped`, `status: false_positive`) to
`pipeline/tickets/archive/` which is off-limits as sources. Do not
look in the archive; do not name archived ids in your output. An
open ticket has `status: new` (or `status: tested` / `fixed` /
`fix_failed`). Key fields:
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

Write each consolidation to a separate JSON file at
`pipeline/staging/consolidation-<slug>.json`. Schema:

```json
{
  "slug": "short-kebab-case-slug",
  "title": "One line summarizing the engine bug, with CR reference where relevant",
  "description": "One paragraph explaining the single engine root cause. Do NOT summarize each source ticket.",
  "engine_path": [
    "file.rs:123 (what this location does)",
    "other.rs:45"
  ],
  "tests": [
    {
      "slug": "snake_case_test_slug",
      "source_ticket": "olivia-01",
      "scenario": "concrete setup + action + assertion"
    },
    {
      "slug": "another_slug",
      "source_ticket": "merged-foo-01",
      "scenario": "..."
    }
  ],
  "also_closes": [
    "merged-fake-widget-target-check-01"
  ]
}
```

Rules enforced by Python:
- `slug` is lowercase-kebab-case.
- Every test slug is snake_case.
- `source_ticket` is **required** on every test and must be one of the
  candidate ticket IDs that were passed in. You cannot invent new
  tests with a null source — the dedup phase composes tests from the
  merged set only. If a scenario is worth testing but isn't already on
  a candidate ticket, the right move is to open a new card ticket via
  audit, not to smuggle it in through a consolidation proposal.
- `also_closes` is optional; omit the key or pass `[]` when you're
  not nesting any merged-* tickets.
- Every test slug in the file must be unique within that file.

Do not write to `pipeline/tickets/` — Python handles the actual
ticket creation and cross-linking. Your job ends with the staging
file(s). If you want to note seed tickets that don't belong in any
cluster, include them in the final agent response text — they are
for human consumption, not machine parsing.

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
