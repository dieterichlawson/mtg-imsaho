## Audit — 2026-04-01

**Scryfall Oracle text**: Brimstone Volley deals 3 damage to any target.
Morbid — Brimstone Volley deals 5 damage to that target instead if a creature died this turn.
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {2}{R}: correct
- Card type Instant: correct
- Target requirement AnyTarget: correct
- Morbid check: state.creature_died_this_turn: correct
- Deals 5 damage if morbid, 3 otherwise: correct
- Uses resolve_damage helper: correct
- Tests exist in tier2_spells.rs and card_mechanics.rs
