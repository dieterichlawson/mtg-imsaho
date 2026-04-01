## Audit — 2026-04-01

**Scryfall Oracle text**: Draw two cards, then discard a card at random.\nFlashback {2}{U}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{R}: correct.
- Type Instant: correct.
- Oracle text matches: draw two, then discard at random.
- Flashback {2}{U}: correct.
- Uses `move_spell_after_resolve`: correct.
- Random discard uses `rand::thread_rng()` and `SliceRandom::choose`: correct.
- Discard event emitted: correct.
- Minor note: discard uses `state.move_object(discard_id, Zone::Graveyard)` directly rather than a dedicated discard helper, but this is consistent with the codebase pattern and the Discarded event is emitted.
- Tests exist in `flashback.rs` (`desperate_ravings_draws_two_discards_one`).
