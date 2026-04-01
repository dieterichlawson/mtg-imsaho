## Audit — 2026-04-01

**Scryfall Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G}
**Scryfall type line**: Sorcery
**Status**: ISSUE

1. **Permanent type choice is hardcoded to "creature"** (`mtg-engine/src/cards/creeping_renaissance.rs`, line 45-46): Oracle says "Choose a permanent type" but code always chooses Creature. The player should be presented with a choice of permanent types (creature, artifact, enchantment, land, planeswalker). The comment on line 12 acknowledges this simplification.
