## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
Creatures enchanted player controls get -1/-1.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({3}{B}{B}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Continuous effect correct**: `ModifyPT { power: -1, toughness: -1, scope: EffectScope::Global(CreatureFilter::AttachedPlayer) }` correctly debuffs all creatures controlled by the enchanted player.

3. **Target requirement correct**: `PlayerOnly` matches "Enchant player."

4. **Resolve correct**: Uses `resolve_curse` helper.

5. **Tests**: No dedicated tests found.
