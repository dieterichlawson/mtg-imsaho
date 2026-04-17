## Candidate tickets to merge

Your seed set is exactly the {num_tickets} ticket(s) listed below —
nothing else is on the table. Your job is to decide which subsets of
them (if any) should become `merged-*` parents.

HARD RULES:
- Do NOT search `pipeline/tickets/` or reference any ticket id that
  isn't in the seed set below. If you are given tickets 1, 2, 3 and
  tickets 4, 5, 6 already exist elsewhere, you may NOT propose
  merging 4 into 5 — those tickets do not exist as far as you're
  concerned.
- Every `source_ticket` on every test you emit MUST match one of the
  seed ids verbatim. Proposals referencing any other id will be rejected.
- You may not invent new tests without a `source_ticket`; merged
  tickets can only consolidate existing tests from the seed set.

{tickets_section}

### Output
Write one consolidation file per proposed merged ticket to
`pipeline/staging/consolidation-<slug>.json` using the schema in the
shared prompt. If no subset is worth merging, produce zero files.
Each file's top-level `slug` must be distinct.
