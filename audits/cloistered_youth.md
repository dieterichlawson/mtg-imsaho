# Audit: Cloistered Youth // Unholy Fiend

## Scryfall Reference
- **Front Face: Cloistered Youth**
  - **Cost:** {1}{W}
  - **Type:** Creature -- Human
  - **Oracle:** At the beginning of your upkeep, you may transform this creature.
  - **P/T:** 1/1

- **Back Face: Unholy Fiend**
  - **Cost:** (none)
  - **Type:** Creature -- Horror
  - **Oracle:** At the beginning of your end step, you lose 1 life.
  - **P/T:** 3/3

## Implementation: `cloistered_youth.rs`
- **Front face name:** Cloistered Youth -- CORRECT
- **Cost:** {1}{W} -- CORRECT
- **Front subtypes:** ["Human"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Unholy Fiend -- CORRECT
- **Back subtypes:** ["Horror"] -- CORRECT
- **Back P/T:** 3/3 -- CORRECT
- **Upkeep:** Transforms front to back -- CORRECT
- **End step:** Loses 1 life when transformed -- CORRECT

## Issues
1. **ISSUE: Front face P/T is 1/1, but Scryfall says 1/1.** Wait -- actually the doc comment says "3/2" on line 6 but the card_data says power: Some(1), toughness: Some(1). Checking Scryfall: front face is 1/1. The doc comment on line 6 says "{1}{W} 3/2 Human" which is WRONG in the comment but the code uses 1/1 which is CORRECT. The dynamic_pt returns (3,3) when transformed which matches the back face. The comment is just misleading but the code is correct.

No functional issues.
