## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
Creatures enchanted player controls attack each combat if able.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({2}{R}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Continuous effect correct**: Uses `ContinuousEffect::ForceAttack` with `EffectScope::Global(CreatureFilter::AttachedPlayer)` to force all creatures controlled by the cursed player to attack.

3. **Target requirement correct**: `PlayerOnly` matches "Enchant player."

4. **Tests**: No dedicated tests found.
