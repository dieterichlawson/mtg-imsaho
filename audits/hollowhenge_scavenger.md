## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — When Hollowhenge Scavenger enters the battlefield, if a creature died this turn, you gain 5 life.
**Scryfall type line**: Creature — Elemental
**Status**: PASS

- Mana cost {3}{G}{G}: correct
- 4/5 stats: correct
- Subtype Elemental: correct
- Morbid ETB: checks creature_died_this_turn flag, gains 5 life if true: correct
- Life gain emits LifeChanged event: correct
- No dedicated tests found, but implementation is straightforward

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: Morbid -- When this creature enters, if a creature died this turn, you gain 5 life.
**Scryfall type line**: Creature -- Elemental
**Status**: PASS

No issues found. Cost {3}{G}{G}, P/T 4/5, Elemental subtype, morbid ETB, LifeChanged event all correct. Note: Scryfall now uses "this creature enters" but code uses "enters the battlefield" -- cosmetic only. Missing dedicated test but implementation is simple.
