## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Enchanted creature gets -2/-2.
**Scryfall type line**: Enchantment — Aura
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({B}), type (Enchantment), subtype (Aura) all match.

2. **Continuous effect correct**: `ModifyPT { power: -2, toughness: -2, scope: EffectScope::Attached }` correctly debuffs the enchanted creature.

3. **Oracle text field**: Only says "Enchanted creature gets -2/-2." and omits "Enchant creature" — minor omission but does not affect functionality.

4. **Resolve correct**: Uses `resolve_aura` helper.

5. **Tests**: No dedicated tests found.
