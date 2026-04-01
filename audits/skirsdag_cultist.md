## Audit — 2026-04-01

**Scryfall Oracle text**: {R}, {T}, Sacrifice a creature: Skirsdag Cultist deals 2 damage to any target.
**Scryfall type line**: Creature — Human Shaman
**Mana cost**: {2}{R}{R}
**P/T**: 2/2
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}{R}{R}, type Creature, subtypes Human/Shaman, P/T 2/2
- Activated ability: {R}, tap, sacrifice a creature, target any target
- Deals 2 damage to creature or player target
- Emits NonCombatDamageDealt events
- Tests: 3 tests in tier8_cards.rs covering damage to creature, damage to player, and inability to activate without a creature

No issues found.
