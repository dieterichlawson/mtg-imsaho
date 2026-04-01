## Audit — 2026-04-01

**Scryfall Oracle text**: {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
**Scryfall type line**: Creature — Human Cleric
**Status**: PASS

- Mana cost {1}{B}: correct.
- Type Creature, subtypes Human Cleric: correct.
- Power/Toughness 1/1: correct.
- Activated ability cost {1} + sacrifice a creature: correct.
- Uses `SacrificeCost::SacrificeCost::SacrificeCreature`: correct.
- Life gain reads toughness from `CreatureDied` event: reasonable approach.
- Life change event emitted: correct.
- `requires_tap: false`: correct (no tap in cost).
- Tests exist in `tier8_cards.rs` (`disciple_of_griselbrand_gains_life`).
