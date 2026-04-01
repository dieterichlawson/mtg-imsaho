## Audit — 2026-04-01

**Scryfall Oracle text (front — Ludevic's Test Subject)**: Defender\n{1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject.
**Scryfall Oracle text (back — Ludevic's Abomination)**: Trample
**Scryfall type line**: Creature — Lizard Egg // Creature — Lizard Horror
**Status**: PASS

- Name (front): Ludevic's Test Subject -- correct
- Cost: {1}{U} -- correct
- Type: Creature -- correct
- Subtypes (front): Lizard, Egg -- correct
- P/T (front): 0/3 -- correct
- Keywords (front): Defender -- correct
- Activated ability: {1}{U} to add hatchling counter, transform at 5 -- correctly implemented using card_state
- Name (back): Ludevic's Abomination -- correct
- Subtypes (back): Lizard, Horror -- correct
- P/T (back): 13/13 -- correct
- Keywords (back): Trample -- correct
- Ability only available on front face -- correct
- Tests exist in tier15_cards.rs

No issues found. Implementation matches Oracle text.

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text (front)**: Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject.
**Scryfall Oracle text (back)**: Trample
**Scryfall type line**: Creature -- Lizard Egg // Creature -- Lizard Horror
**Status**: PASS

No issues found. Hatchling counters tracked via card_state. Transform at 5 counters. Back face 13/13 Trample. Ability only available on front face (untransformed).
