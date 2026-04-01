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

## Audit — 2026-04-01

**Scryfall Oracle text**: {R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
**Scryfall type line**: Creature — Human Shaman
**Status**: ISSUE

1. **Missing `damaged_by` tracking** (skirsdag_cultist.rs:54-57): When dealing 2 damage to a creature target, the code increments `damage_marked` and emits `NonCombatDamageDealt` but does NOT push to `obj.damaged_by`. Other damage-dealing cards (e.g. blazing_torch.rs, harvest_pyre.rs, helpers.rs) all do `obj.damaged_by.push(source_id)`. This could affect interactions that check what damaged a creature.
