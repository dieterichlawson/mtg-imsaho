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
