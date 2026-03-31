# Audit: Midnight Haunting

## Official Oracle
- **Name:** Midnight Haunting
- **Cost:** {2}{W}
- **Type:** Instant
- **Oracle:** Create two 1/1 white Spirit creature tokens with flying.

## Implementation: `mtg-engine/src/cards/midnight_haunting.rs`
- **Name:** Midnight Haunting -- CORRECT
- **Cost:** {2}{W} -- CORRECT
- **Type:** Instant -- CORRECT
- **on_resolve:** Creates two 1/1 white Spirit tokens with flying -- CORRECT

## Issues
1. **Token subtypes missing:** Uses `create_token("Spirit", ...)` which passes empty subtypes vec. The Spirit tokens will not have the "Spirit" creature subtype. Should use `create_token_with_subtypes` with `vec!["Spirit".into()]`.

## Verdict
**FAIL** -- 1 issue: Spirit tokens lack "Spirit" creature subtype.
