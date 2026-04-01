## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: PASS

### Findings

1. **Card data correct**: Name, cost ({3}{B}), type (Enchantment), subtypes (Aura, Curse) all match.

2. **Upkeep trigger correct**: Triggers on enchanted player's upkeep, verified by checking `active_player == cursed_player`.

3. **Exile logic correct**: When 2 or fewer cards in graveyard, exiles all. When more, presents a choice to the cursed player. Uses `PendingEffect::ExileCurseOfOblivion { remaining: 1 }` to handle the second exile pick.

4. **Player choice implemented**: The cursed player gets to choose which cards to exile, which is correct.

5. **Tests**: No dedicated tests found.
