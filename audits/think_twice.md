## Audit — 2026-04-01

**Scryfall Oracle text**: Draw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Scryfall type line**: Instant
**Status**: PASS

- Name: correct ("Think Twice")
- Cost: {1}{U} -- correct
- Type: Instant -- correct
- Oracle text: matches
- Flashback cost: {2}{U} -- correct
- On resolve: draws 1 card -- correct
- Tests exist in `flashback.rs`, `innistrad_simple_cards.rs`, `tier12_cards.rs`
- No issues found
