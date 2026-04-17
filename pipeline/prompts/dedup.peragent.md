## Candidate tickets to merge

Consider merging ONLY the {num_tickets} ticket(s) listed below. Do NOT
search `pipeline/tickets/` for other tickets and do NOT propose merges
that pull in tickets outside this seed set — even if you notice
related open tickets. The human chose this exact group; your job is
to decide which subsets of it (if any) should become `merged-*`
parents.

{tickets_section}

### Output
Write one consolidation file per proposed merged ticket to
`pipeline/staging/consolidation-<slug>.json` using the schema in the
shared prompt. Each test's `source_ticket` MUST be one of the seed
ticket ids above. If no subset is worth merging, produce zero files.
Each file's top-level `slug` must be distinct.
