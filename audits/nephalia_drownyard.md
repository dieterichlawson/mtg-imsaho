# Audit: Nephalia Drownyard

## Official Oracle
- **Name:** Nephalia Drownyard
- **Type:** Land
- **Oracle:** {T}: Add {C}. {1}{U}{B}, {T}: Target player mills three cards.

## Implementation: `mtg-engine/src/cards/nephalia_drownyard.rs`
- **Name:** Nephalia Drownyard -- CORRECT
- **Type:** Land -- CORRECT
- **Mana ability:** {T}: Add {C} -- CORRECT
- **Activated ability:** {1}{U}{B}, {T}, targets player -- CORRECT
- **on_activate_ability:** Mills 3 cards via engine::mill_cards -- CORRECT

## Verdict
**PASS** -- No issues found.
