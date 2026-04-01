## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.\n{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
**Scryfall type line**: Land
**Status**: ISSUE

- Name: Correct ("Moorland Haunt")
- Cost: None (Land) - Correct
- Type: Land - Correct
- Oracle text matches.
- Mana ability: {T}: Add {C} - Correct
- Activated ability: {W}{U}, {T}, Exile a creature card from graveyard - Correct cost, correct tap requirement

Issues:
1. **No player choice for which creature to exile**: The implementation auto-picks the first creature card in the graveyard (`next()`) rather than letting the player choose which creature card to exile. The Oracle text says "Exile a creature card from your graveyard" which implies the controller chooses.

- Tests: innistrad_simple_cards.rs has `moorland_haunt_card_data` and `moorland_haunt_creates_spirit_token` tests.
