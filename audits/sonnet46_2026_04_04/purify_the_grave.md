## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Exile target card from a graveyard.
Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Flashback exile on resolution**: When cast from graveyard, `is_flashback = true`, `cast_with_flashback = true` is set on the object, and `move_spell_after_resolve` uses this flag to exile instead of graveyard. Correctly implemented. PASS
- **Flashback exile when countered**: Counterspell's `on_resolve` calls `state.move_spell_after_resolve(target_id)` on the countered spell, which checks `cast_with_flashback` and correctly exiles it. Ruling "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered" is satisfied. PASS
- **Flashback exile when fizzled (all targets illegal)**: `stack.rs` fizzle path calls `state.move_spell_after_resolve(object_id)`, which also checks `cast_with_flashback`. Correctly exiles the fizzled flashback spell. PASS
- **Target is any graveyard ("from a graveyard")**: `TargetRequirement::GraveyardCard` gathers all objects in `Zone::Graveyard` without filtering by owner. Both own and opponent graveyards are targetable. PASS
- **Target legality at resolution**: `is_target_legal` in `stack.rs` checks `obj.zone == Zone::Graveyard` for `GraveyardCard` targets. If the target card is moved out of the graveyard before resolution, the spell fizzles and the card is still exiled (flashback). PASS
- **Flashback cost is {W}**: Code sets `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::White)]))`. Matches oracle text `Flashback {W}`. PASS
- **Normal mana cost is {W}**: Code sets `cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::White)]))`. Matches oracle text `{W}`. PASS
- **Keywords vec empty (Flashback not in engine Keyword enum)**: The engine's `Keyword` enum contains only keyword abilities affecting game rules (Flying, First Strike, Hexproof, etc.). Flashback is a keyword ability implemented via the `flashback_cost` field, not the `keywords` vec. Empty `keywords` vec is correct. PASS
- **"target card" — no type restriction**: `GraveyardCard` matches all objects in any graveyard regardless of card type (not just creatures or spells). Oracle says "target card" with no type restriction. PASS
- **Purify the Grave cannot target itself via flashback**: When Purify the Grave is cast from the graveyard, it moves to the stack (`Zone::Stack`) before the targets are recorded (lines 1632–1635 of engine.rs). At resolution, `is_target_legal` checks `obj.zone == Zone::Graveyard`; Purify the Grave itself is no longer in the graveyard, so it cannot exile itself. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic exile of graveyard card on resolution: `tests/tier11_cards.rs:253` (`purify_the_grave_exiles_card_from_graveyard`)
- Flashback cost exists ({W}): `tests/tier11_cards.rs:268` (`purify_the_grave_has_flashback`)
- Flashback spell exiled after resolution (generic engine test, Geistflame): `tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`)
- Flashback spell exiled when countered (generic engine test, Geistflame): `tests/flashback.rs:128` (`flashback_spell_countered_is_exiled`)
- Purify the Grave specifically cast via flashback and exiled: NOT TESTED (covered generically by flashback.rs tests)
- Target legality check at resolution (fizzle when target removed): NOT TESTED for Purify the Grave specifically
- Purify the Grave targeting own graveyard card (not just opponent's): NOT TESTED explicitly
