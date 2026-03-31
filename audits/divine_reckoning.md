# Audit: Divine Reckoning

## Reference (Scryfall)
- **Name:** Divine Reckoning
- **Cost:** {2}{W}{W}
- **Type:** Sorcery
- **Oracle:** Each player chooses a creature they control. Destroy the rest. Flashback {5}{W}{W}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{W}{W})
- Type: CORRECT (Sorcery)
- Oracle text: CORRECT (says "destroys" matching Scryfall "Destroy the rest")
- Flashback cost: CORRECT ({5}{W}{W})
- Each player keeps one creature: CORRECT
- Destroys the rest (not sacrifice): CORRECT (uses try_destroy)
- P/T: CORRECT (N/A)

## Issues
None found.
