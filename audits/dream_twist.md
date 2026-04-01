## Audit — 2026-04-01

**Scryfall Oracle text**: Target player mills three cards.\nFlashback {1}{U}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {U}: correct.
- Type Instant: correct.
- Flashback {1}{U}: correct.
- Targets a player (PlayerOnly): correct.
- Mills 3 cards via `mill_cards`: correct.
- Uses `move_spell_after_resolve`: correct.
- Tests exist in `flashback.rs` (`dream_twist_mills_three`).
