## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever Kessig Cagebreakers attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
**Scryfall type line**: Creature — Human Rogue
**Status**: PASS

- Name: Kessig Cagebreakers -- correct
- Cost: {4}{G} -- correct
- Type: Creature -- correct
- Subtypes: Human, Rogue -- correct
- P/T: 3/4 -- correct
- Triggered ability: attacks trigger creating Wolf tokens tapped and attacking -- correctly implemented
- Token creation: 2/2 green Wolf creature tokens -- correct
- Counts creature cards in graveyard -- correct
- Sets tokens as tapped and attacking the correct defending player -- correct
- Tests exist in tier15_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Whenever Kessig Cagebreakers attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
**Scryfall type line**: Creature -- Human Rogue
**Status**: PASS

No issues found. Token creation uses create_token_with_subtypes with "Wolf" subtype. Tokens set tapped and attacking. Counts creature cards in graveyard correctly. Scryfall ruling notes tokens were never "declared as attacking" (relevant for "whenever a creature attacks" triggers) -- the code correctly inserts into combat.attackers without firing attack triggers on tokens.
