## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({2}{U}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Upkeep trigger correct**: Triggers on enchanted player's upkeep.

3. **Mill correct**: Uses `crate::engine::mill_cards(state, cursed_player, 2)` to mill 2 cards.

4. **Tests**: No dedicated tests found.
