## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\n{U}: Return Lantern Spirit to its owner's hand.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

- Name: Lantern Spirit -- correct
- Cost: {2}{U} -- correct
- Type: Creature -- correct
- Subtypes: Spirit -- correct
- P/T: 2/1 -- correct
- Keywords: Flying -- correct
- Activated ability: {U} to return to hand -- correctly implemented
- Ability does not require tap -- correct
- Tests exist in activated_abilities.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Flying. {U}: Return Lantern Spirit to its owner's hand.
**Scryfall type line**: Creature -- Spirit
**Status**: PASS

No issues found. Note: Scryfall says "Return this creature to its owner's hand" in current oracle text template. The code correctly uses move_object to Zone::Hand.
