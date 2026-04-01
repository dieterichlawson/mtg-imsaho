## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: Correct ("Paraselene")
- Cost: {2}{W} - Correct
- Type: Sorcery - Correct
- Oracle text matches.
- Implementation: Finds all enchantments on battlefield, attempts to destroy each via try_destroy, counts successful destructions, gains that much life. Correct.
- Properly uses try_destroy (which handles indestructible). Correct.
- Life gain is properly tracked with LifeChanged event. Correct.
- Tests: innistrad_simple_cards.rs has `paraselene_card_data` and `paraselene_destroys_enchantments_and_gains_life`.

No issues found.
