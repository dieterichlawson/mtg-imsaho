## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Rolling Temblor deals 2 damage to each creature without flying.
Flashback {4}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying granted via aura (e.g., Spectral Flight's `GrantKeyword { keyword: Keyword::Flying }`): `has_keyword` checks `GrantKeyword` continuous effects via `has_continuous_effect`, so aura-granted Flying is correctly detected and those creatures are excluded from damage. PASS
- Reach creatures without Flying: Code checks `Keyword::Flying` specifically, so reach creatures (which lack Flying) are correctly dealt 2 damage. PASS
- Token flyers: `has_keyword` checks `obj.keywords` first (line 1000 of state.rs), so tokens with Flying set directly on the object are correctly detected and excluded. PASS
- Flashback exile after countering: Counterspell calls `state.move_spell_after_resolve(*target_id)` (counterspell.rs line 50), which checks `cast_with_flashback` and moves to exile. PASS
- Flashback exile after resolution: `move_spell_after_resolve` is called at rolling_temblor.rs line 47; it correctly checks `cast_with_flashback` and moves to Zone::Exile if true. PASS
- Sorcery timing for flashback: Engine enforces `is_sorcery_speed` (main phase, stack empty, active player's turn) for Sorcery-type flashback spells at engine.rs lines 693-706. PASS
- `cast_with_flashback` flag set on cast: engine.rs line 1636-1638 sets `obj.cast_with_flashback = true` when `is_flashback` is true (i.e., spell in graveyard and not a "cast from graveyard" card). PASS
- NonCombatDamageDealt used (not CombatDamageDealt): Code emits `GameEvent::NonCombatDamageDealt` at rolling_temblor.rs line 40. PASS
- Past in Flames interaction: Past in Flames grants dynamic flashback via `until_end_of_turn_flashback`; engine prefers dynamic flashback cost over card's own `flashback_cost` (engine.rs line 676-679), so Rolling Temblor would be castable at {2}{R}. Correct per Past in Flames oracle. PASS
- Flashback spell countered still exiled: Both Counterspell (counterspell.rs:50) and the fizzle path in stack.rs (line 84) use `move_spell_after_resolve`, which checks `cast_with_flashback` and exiles. PASS
- Mana cost {2}{R}: Code uses `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Red)])`. PASS
- Flashback cost {4}{R}{R}: Code uses `ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])`. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Deals 2 damage to each creature without flying (basic function): `flashback.rs:278` (`rolling_temblor_damages_non_flyers`) — TESTED
- Flyers take 0 damage: `flashback.rs:278` (`rolling_temblor_damages_non_flyers`) — TESTED
- Flashback exiles the spell after resolution: NOT TESTED for Rolling Temblor specifically (tested for other flashback spells at `flashback.rs:86` via Geistflame)
- Flashback casting Rolling Temblor from graveyard: NOT TESTED (only normal hand cast tested)
- Flashback spell countered is exiled: NOT TESTED for Rolling Temblor specifically (tested for Geistflame at `flashback.rs:129`)
- Aura-granted Flying creatures excluded from damage: NOT TESTED
- Reach creatures take damage (no flying): NOT TESTED
- Sorcery timing enforced for flashback: NOT TESTED for Rolling Temblor specifically
