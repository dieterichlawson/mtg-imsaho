## Audit — 2026-04-01

**Scryfall Oracle text**: Sulfur Falls enters the battlefield tapped unless you control an Island or a Mountain.\n{T}: Add {U} or {R}.
**Scryfall type line**: Land
**Status**: PASS

- Name: correct ("Sulfur Falls")
- Cost: None (land) -- correct
- Type: Land -- correct
- ETB tapped logic: checks for Island or Mountain subtypes among other controlled permanents (excluding self) -- correct
- Mana abilities: produces {U} or {R} -- correct
- Two separate ManaAbilityDef entries for the two color choices -- correct
- Tests exist in `innistrad_simple_cards.rs`
- No issues found
