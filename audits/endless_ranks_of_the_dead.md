## Audit — 2026-04-01

**Scryfall Oracle text**: At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
**Scryfall type line**: Enchantment
**Status**: PASS

- Mana cost {2}{B}{B}: correct.
- Type Enchantment: correct.
- Triggered ability on upkeep (TriggerKind::Upkeep): correct.
- Only triggers on controller's upkeep (`state.active_player != controller` guard): correct.
- Counts Zombies via card data subtypes and object subtypes: correct.
- X = zombie_count / 2 (integer division = rounded down): correct.
- Creates 2/2 black Zombie creature tokens with correct subtypes: correct.
- Tests exist in `tier7_cards.rs` (`endless_ranks_creates_zombie_tokens`).
