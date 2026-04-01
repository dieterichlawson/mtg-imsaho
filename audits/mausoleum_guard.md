## Audit — 2026-04-01

**Scryfall Oracle text**: When Mausoleum Guard dies, create two 1/1 white Spirit creature tokens with flying.
**Scryfall type line**: Creature — Human Scout
**Status**: PASS

- Name: Mausoleum Guard -- correct
- Cost: {3}{W} -- correct
- Type: Creature -- correct
- Subtypes: Human, Scout -- correct
- P/T: 2/2 -- correct
- Triggered ability: on death, create two 1/1 white Spirit tokens with flying -- correctly implemented
- Tokens have correct stats (1/1), color (white), type (Creature), keyword (Flying), subtype (Spirit) -- correct
- Uses controller (not owner) for token creation -- correct
- Tests exist in tier3_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01

**Scryfall Oracle text**: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
**Scryfall type line**: Creature — Human Scout
**Status**: PASS

No issues found.
