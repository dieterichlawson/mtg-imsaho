## Ticket to test

{ticket_body}

### Oracle text (pre-fetched from Scryfall, if available for a single card)

{oracle}
{engine_note}
### Test file
Write every test for this ticket to:
`mtg-engine/tests/pipeline_bugs_{tid_snake}.rs`

### Staging output
Write your result to `pipeline/staging/{tid}-test.json` matching the schema
in the shared prompt. One entry per slug in the ticket's `## Tests` section.

### Commit
Commit the test file with a descriptive message before writing the staging
output — the worktree must be clean.

### Ticket ID: {tid}
