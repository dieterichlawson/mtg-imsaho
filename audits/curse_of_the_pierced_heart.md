## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, Curse of the Pierced Heart deals 1 damage to that player or a planeswalker that player controls.
**Scryfall type line**: Enchantment — Aura Curse
**Status**: ISSUE

1. **Oracle text in code is outdated** (`mtg-engine/src/cards/curse_of_the_pierced_heart.rs`, line 27): Code oracle text says "deals 1 damage to that player" but current Scryfall Oracle text says "deals 1 damage to that player or a planeswalker that player controls." The planeswalker targeting option is missing.
2. **No planeswalker damage option** (`mtg-engine/src/cards/curse_of_the_pierced_heart.rs`, lines 62-72): The upkeep handler always damages the player. Per Oracle, the controller of the Curse should choose whether to deal 1 damage to the player or a planeswalker that player controls. This is minor in a format without many planeswalkers but is technically incorrect.
