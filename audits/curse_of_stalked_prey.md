## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({1}{R}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Trigger correct**: Uses `TriggerKind::AnyCombatDamageToPlayer` which correctly matches "deals combat damage."

3. **Target verification correct**: Checks that `damaged_player` matches the `cursed_player` (attached_to_player).

4. **Counter placement correct**: Adds +1/+1 counter to the source creature that dealt damage, only if it's still on the battlefield.

5. **Tests**: No dedicated tests found.
