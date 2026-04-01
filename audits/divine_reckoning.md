## Audit — 2026-04-01

**Scryfall Oracle text**: Each player chooses a creature they control, then sacrifices the rest.\nFlashback {5}{W}{W}
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Mana cost {2}{W}{W}: correct.
- Type Sorcery: correct.
- Flashback {5}{W}{W}: correct.
- Uses `move_spell_after_resolve`: correct.

**Issue 1 — Oracle says "sacrifices the rest" but implementation uses `try_destroy`.**
The Oracle text says each player "sacrifices" the rest of their creatures, not "destroys" them. The implementation oracle_text field also incorrectly says "destroys the rest" instead of "sacrifices the rest." Sacrifice bypasses indestructible and regeneration, while destroy does not. The implementation uses `crate::destruction::try_destroy` on line 74, which respects indestructible — this is wrong. The card should use a sacrifice mechanic. The code comment on line 67 also says "not sacrifice — respects indestructible" which contradicts the Oracle text.

**Issue 2 — Player choice is automated (picks highest toughness) instead of allowing player to choose.** This is noted as a simplification in the code comment and may be acceptable for the engine's current state, but it's technically incorrect.

- Tests exist in `tier8_cards.rs` (`divine_reckoning_keeps_one_per_player`, `divine_reckoning_with_one_creature_keeps_it`, `divine_reckoning_has_flashback`).
