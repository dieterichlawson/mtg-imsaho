## Audit — 2026-04-01

**Scryfall Oracle text**: {T}: Add {C}.\n{1}{U}{B}, {T}: Target player mills three cards.
**Scryfall type line**: Land
**Status**: PASS

- Name: Correct ("Nephalia Drownyard")
- Cost: None (Land) - Correct
- Type: Land - Correct
- Oracle text matches.
- Mana ability: {T}: Add {C} - Correct
- Activated ability: {1}{U}{B}, {T}: Target player mills three cards - Correct cost, correct tap, targets a player, mills 3.
- on_activate_ability calls mill_cards(state, player, 3). Correct.
- Tests: tier10_cards.rs has `nephalia_drownyard_card_data` and `nephalia_drownyard_mills_three`.

No issues found.
