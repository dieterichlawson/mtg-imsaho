## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature gets +4/+2.
Equip — Sacrifice a creature.
**Scryfall type line**: Artifact — Equipment
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({4}), type (Artifact), subtype (Equipment) all match.

2. **Continuous effect correct**: `ModifyPT { power: 4, toughness: 2, scope: EffectScope::Attached }` matches "+4/+2."

3. **Equip cost correct**: `SacrificeCost::SacrificeCreature` with no mana cost, sorcery speed. Matches "Equip — Sacrifice a creature."

4. **Equip target restriction correct**: Only targets creatures you control.

5. **on_resolve sets is_equipment**: Correct.

6. **Tests**: No dedicated tests found.
