## Audit — 2026-04-01

**Scryfall Oracle text**: Counter target spell.
**Scryfall type line**: Instant
**Status**: ISSUE

### Findings

1. **Countered spell goes to graveyard via move_spell_after_resolve (minor ISSUE)**: Line 50 uses `move_spell_after_resolve(*target_id)` for the countered spell. This should be correct if `move_spell_after_resolve` sends spells to the graveyard, which is the standard behavior for countered spells. However, technically a countered spell should be removed from the stack and put into the graveyard (not "resolved"), and `move_spell_after_resolve` may have special behavior for flashback cards (exile instead of graveyard). For a countered flashback spell, the card should go to the graveyard (not exile), since it was countered not resolved. If `move_spell_after_resolve` exiles flashback spells, this is a bug.

2. **Card data correct**: Name, cost ({U}{U}), type (Instant) all match.

3. **Target validation correct**: Checks target is on the stack (line 37).

4. **Stack cleanup**: Correctly removes the countered spell from the stack (line 49).

5. **Uses move_spell_after_resolve for self**: Correct (line 55).

6. **Tests**: Found in `bug_fixes.rs`, `fizzle.rs`, `flashback.rs`, `spell_fizzle.rs`, `spells.rs`.
