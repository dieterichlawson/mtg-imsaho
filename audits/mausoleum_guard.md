# Audit: Mausoleum Guard

## Official Oracle
- **Name:** Mausoleum Guard
- **Cost:** {3}{W}
- **Type:** Creature — Human Scout
- **Oracle:** When Mausoleum Guard dies, create two 1/1 white Spirit creature tokens with flying.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/mausoleum_guard.rs`
- **Name:** Mausoleum Guard -- CORRECT
- **Cost:** {3}{W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Human, Scout -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Triggered ability:** SelfDies -- CORRECT
- **on_dies:** Creates two 1/1 white Spirit tokens with flying -- CORRECT

## Issues
1. **Token subtypes missing:** Uses `create_token("Spirit", ...)` which passes empty subtypes vec. The Spirit tokens will not have the "Spirit" creature subtype. Should use `create_token_with_subtypes` with `vec!["Spirit".into()]`.

## Verdict
**FAIL** -- 1 issue: Spirit tokens lack "Spirit" creature subtype.
