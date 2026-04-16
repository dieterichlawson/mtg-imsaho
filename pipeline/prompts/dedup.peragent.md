## Candidate tickets (seed set)

The following {num_tickets} ticket(s) are proposed as the starting point for
your dedup analysis. You are NOT required to merge every one of them, and you
SHOULD search the full `pipeline/tickets/` directory for other open tickets
(card tickets or existing `merged-*` tickets) that belong in the same clusters.

{tickets_section}

### Output
Write one consolidation file per proposed merged ticket to
`pipeline/staging/consolidation-<slug>.json` using the schema in the shared
prompt. If no tickets should be merged, produce zero files. Each file's
top-level `slug` must be distinct.
