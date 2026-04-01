## Audit — 2026-04-01

**Scryfall Oracle text**: Tap up to two target creatures.\nFlashback {1}{U}
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{W}: correct.
- Type Instant: correct.
- Flashback {1}{U}: correct.
- Targets up to 2 creatures via `UpToTargets(2, Creature)`: correct.
- Taps each target creature on resolve: correct.
- Checks zone == Battlefield before tapping: correct.
- Uses `move_spell_after_resolve`: correct.
- Tests exist in `flashback.rs` (`feeling_of_dread_taps_creature`) and `card_mechanics.rs` (`feeling_of_dread_taps_two`).
