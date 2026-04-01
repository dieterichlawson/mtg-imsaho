## Audit — 2026-04-01

**Scryfall Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Scryfall type line**: Instant
**Status**: PASS

- Mana cost {1}{U}{U}: correct.
- Type Instant: correct.
- Targets a spell on the stack: correct.
- On resolve: removes from stack, moves to Exile (not graveyard): correct.
- Uses `move_spell_after_resolve` for the Dissipate spell itself: correct.
- `is_valid_target` checks zone == Stack: correct.
- Tests exist in `tier2_spells.rs` (`dissipate_counters_and_exiles`).

## Audit — 2026-04-01

**Scryfall Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Scryfall type line**: Instant
**Status**: PASS

No issues found.
