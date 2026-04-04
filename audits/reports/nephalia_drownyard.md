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

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Nephalia Drownyard  
**Type**: Land  
**Oracle text**: "{T}: Add {C}.\n{1}{U}{B}, {T}: Target player mills three cards."

### Checks
- Name: "Nephalia Drownyard" -- PASS
- Cost: None (land) -- PASS
- Type: Land -- PASS
- Mana ability: {T}: Add {C}, requires untapped on battlefield -- PASS
- Activated ability cost: {1}{U}{B}, requires tap -- PASS
- Activated ability target: PlayerOnly -- PASS
- Behavior: Calls `mill_cards(state, player_id, 3)` -- mills 3 cards -- PASS

**Verdict: PASS**
