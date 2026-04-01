## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
**Scryfall type line**: Enchantment — Aura
**Status**: ISSUE

1. **"You may" draw not presented as optional** (`mtg-engine/src/cards/curiosity.rs`, line 65): The code auto-draws when triggered. Oracle says "you may draw a card" — while auto-drawing is almost always correct in 2-player (as noted in the comment), strictly speaking this should be optional. In edge cases (e.g., near-decking), the player might decline to draw.
