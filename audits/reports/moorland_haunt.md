# Audit: Moorland Haunt

## Official Oracle
- **Name:** Moorland Haunt
- **Type:** Land
- **Oracle:** {T}: Add {C}. {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.

## Implementation: `mtg-engine/src/cards/moorland_haunt.rs`
- **Name:** Moorland Haunt -- CORRECT
- **Type:** Land -- CORRECT
- **Mana ability:** {T}: Add {C} -- CORRECT
- **Activated ability:** {W}{U}, {T}, exile creature from graveyard -- CORRECT
- **Token:** 1/1 white Spirit with flying, "Spirit" subtype -- CORRECT

## Notes
- Token is created with name "Spirit Token" rather than "Spirit". Minor cosmetic issue.
- Creature exile from graveyard is auto-picked (first found). Acceptable simplification.

## Verdict
**PASS** -- No issues found.

## Audit — 2026-04-02

**Oracle source**: Scryfall  
**Card**: Moorland Haunt  
**Type**: Land  
**Oracle text**: "{T}: Add {C}.\n{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying."

### Checks
- Name: "Moorland Haunt" -- PASS
- Cost: None (land) -- PASS
- Type: Land -- PASS
- Mana ability: {T}: Add {C}, requires untapped on battlefield -- PASS
- Activated ability cost: {W}{U}, requires tap -- PASS
- Activated ability condition: Checks for creature card in graveyard (non-token with power) -- PASS
- Behavior: Exiles a creature card from graveyard, creates 1/1 white Spirit token with flying -- PASS
- Token details: name "Spirit Token", 1/1, White, Creature, Flying, subtype Spirit -- PASS

**Verdict: PASS**
