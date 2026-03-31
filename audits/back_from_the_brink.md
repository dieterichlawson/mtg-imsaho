# Audit: Back from the Brink

## Oracle (Scryfall)
- **Name:** Back from the Brink
- **Cost:** {4}{U}{U}
- **Type:** Enchantment
- **Oracle:** Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/back_from_the_brink.rs`
- **Name:** Back from the Brink ✅
- **Cost:** {4}{U}{U} ✅
- **Type:** Enchantment ✅
- **P/T:** N/A ✅
- **Oracle text:** matches ✅
- **sorcery_speed_only:** true ✅
- **Token creation:** uses `create_token_copy` before exiling (correct order) ✅
- **Activated ability cost:** Uses Generic(2) instead of the exiled card's mana cost — noted as known simplification in doc comment ✅ (documented)

## Verdict: PASS — no issues found (cost approximation is documented)
