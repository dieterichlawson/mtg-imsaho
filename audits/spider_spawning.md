## Audit — 2026-04-01

**Scryfall Oracle text**: Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.\nFlashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Sorcery
**Status**: PASS

- Name: correct ("Spider Spawning")
- Cost: {4}{G} -- correct
- Type: Sorcery -- correct
- Oracle text: matches
- Flashback cost: {6}{B} -- correct
- Token creation: 1/2 green Spider with reach -- correct (uses `create_token_with_subtypes` with proper params)
- Correctly counts creature cards in graveyard (excluding itself while on the stack)
- Tests exist in `tier5_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.
Flashback {6}{B}
**Scryfall type line**: Sorcery
**Status**: PASS

No issues found. Correctly counts creatures in graveyard (excluding self on stack), creates tokens with subtypes, flashback cost correct, uses `move_spell_after_resolve`.
