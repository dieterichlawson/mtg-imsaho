## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Put target creature on top of its owner's library.
Flashback {7}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Flashback exile after resolution**: `move_spell_after_resolve` checks `cast_with_flashback` and sends the spell to exile if true. `cast_with_flashback` is set to `true` in `engine.rs` when a spell is cast from the graveyard via flashback. PASS
- **Flashback spell countered → exile**: `counterspell.rs` calls `state.move_spell_after_resolve(*target_id)` on the countered spell, which correctly checks `cast_with_flashback` and exiles it. PASS (test `flashback_spell_countered_is_exiled` confirms this system-wide)
- **Flashback spell fizzled (target illegal) → exile**: `stack.rs` `resolve_spell` calls `state.move_spell_after_resolve(object_id)` when fizzling, correctly exiling a flashback spell whose target became illegal. PASS
- **Sorcery timing enforced on flashback**: `engine.rs` lines 692–706 gate flashback casts of sorceries behind `is_sorcery_speed` (main phase, empty stack, your turn). PASS
- **"its owner's library" (owner not controller)**: `on_resolve` reads `obj.owner` to determine whose library receives the creature. PASS
- **"on top of" (insert at position 0)**: `library_order.insert(0, *target_id)` puts the creature at index 0, which is the top of the library. PASS
- **Target becomes illegal before resolution (zone check in on_resolve)**: `on_resolve` has a secondary guard `if obj.zone == Zone::Battlefield`; the primary check is `is_target_legal` in `resolve_spell`, which returns false if the creature is no longer on the battlefield, causing a fizzle. PASS
- **Hexproof on target creature**: `can_be_targeted` in `engine.rs` correctly prevents targeting hexproof creatures controlled by opponents. PASS
- **Flashback not offered without sufficient mana**: `engine.rs` checks `mana::can_pay(&player_state.mana_pool, fb_cost)` before generating flashback cast actions. PASS
- **Mana cost {3}{U}**: `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Blue)])` matches oracle. PASS
- **Flashback cost {7}{U}**: `ManaCost::new(vec![ManaSymbol::Generic(7), ManaSymbol::Colored(Color::Blue)])` matches oracle. PASS
- **Flashback keyword not in Keyword enum**: Flashback is implemented via the dedicated `flashback_cost` field in `CardData`; the engine's `Keyword` enum only covers keyword abilities like Flying, Haste, etc. Absence of Flashback from `keywords: vec![]` is correct and not an issue. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Put target creature on top of its owner's library (basic effect): `mtg-engine/tests/tier11_cards.rs:280` (`grasp_of_phantoms_puts_creature_on_top_of_library`) — TESTED
- Flashback cost exists: `mtg-engine/tests/tier11_cards.rs:296` (`grasp_of_phantoms_has_flashback`) — TESTED
- Flashback spell exiled after resolution: NOT TESTED for Grasp specifically; covered by system test `mtg-engine/tests/flashback.rs:86` (`flashback_spell_is_exiled_after_resolve`) using Geistflame
- Flashback spell exiled when countered: `mtg-engine/tests/flashback.rs:129` (`flashback_spell_countered_is_exiled`) — TESTED (system-wide, not Grasp-specific)
- Flashback spell exiled when fizzled: NOT TESTED for any flashback card
- Sorcery timing restriction for flashback: NOT TESTED
- "its owner's library" (owner vs controller distinction): NOT TESTED
- "on top of" (position 0 in library_order): `mtg-engine/tests/tier11_cards.rs:292` — TESTED (asserts `library_order[0] == target_creature`)
- Flashback not offered without enough mana: `mtg-engine/tests/flashback.rs:65` (`flashback_not_offered_without_mana`) — TESTED (system-wide)
