## Audit — 2026-04-01

**Scryfall Oracle text**: Target player reveals their hand. You choose a nonland card from it. That player discards that card.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Name: Correct ("Night Terrors")
- Cost: {2}{B} - Correct
- Type: Sorcery - Correct
- Target: Player - Correct

Issues:
1. **Exiles instead of discards**: The Oracle text says "That player discards that card" but the implementation exiles the card (`state.move_object(exile_id, Zone::Exile)`). It should move the card to the graveyard (Zone::Graveyard), not exile. The implementation oracle_text also incorrectly says "exile that card" instead of "discards that card".
2. **Auto-selection**: The implementation auto-picks the first nonland card rather than presenting a choice to the caster. However, this is a common simplification in this codebase.

- Tests: tier11_cards.rs has `night_terrors_exiles_nonland_from_hand` and `night_terrors_skips_lands`. Note the test name itself says "exiles" which confirms the bug is baked into the test.
