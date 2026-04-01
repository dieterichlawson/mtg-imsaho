## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, Sacrifice Traveler's Amulet: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
**Scryfall type line**: Artifact
**Status**: PASS

- Name: correct ("Traveler's Amulet")
- Cost: {1} -- correct
- Type: Artifact -- correct
- Activated ability: {1}, Sacrifice -- correct (SacrificeCost::SacrificeThis, no tap required)
- Searches library for a basic land (checks CardType::Land and Supertype::Basic) -- correct
- Puts the land into hand -- correct
- Shuffle noted as no-op in engine (acceptable engine limitation)
- Tests exist in `tier9_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
**Scryfall type line**: Artifact
**Mana cost**: {1}
**Status**: PASS

No issues found.
