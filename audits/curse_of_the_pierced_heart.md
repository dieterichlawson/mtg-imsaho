## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, Curse of the Pierced Heart deals 1 damage to that player.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({1}{R}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Upkeep trigger correct**: Triggers on enchanted player's upkeep.

3. **Damage implementation correct**: Deals 1 damage (subtracts life and emits `NonCombatDamageDealt` event). This is damage, not life loss, which matches "deals 1 damage."

4. **NonCombatDamageDealt event correct**: Correctly uses `NonCombatDamageDealt` rather than `CombatDamageDealt`.

5. **Tests**: No dedicated tests found.
