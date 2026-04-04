## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Draw a card.
Flashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Flashback exile after resolution**: `move_spell_after_resolve` in `state.rs:1132` checks `obj.cast_with_flashback` and sends to `Zone::Exile` when true. `cast_with_flashback` is set to `true` in `engine.rs:1637` when casting from graveyard (and card has `flashback_cost`). PASS
- **Flashback exile when countered**: All counter paths in `engine.rs` (lines 1962, 1990, 1993, 2035, 2083, 2214, 2335) and the fizzle path in `stack.rs:84` all call `move_spell_after_resolve`, correctly routing flashback spells to exile. PASS
- **Flashback exile when fizzled (all targets illegal)**: `stack.rs:84` calls `move_spell_after_resolve` before returning, so a fizzled flashback Think Twice is also exiled. PASS (Think Twice has no targets, so fizzle is moot, but the path is correct in general.)
- **Timing restriction for instant-speed flashback**: `engine.rs:692-706` checks card type when generating flashback actions. Think Twice is `CardType::Instant`, so `can_cast_timing` is `true` unconditionally, allowing flashback at any time (not just sorcery speed). PASS
- **Keywords vec empty (Flashback not listed)**: Flashback is not in the engine's `Keyword` enum (`types.rs:289-305`); it is implemented via the `flashback_cost: Option<ManaCost>` field on `CardData`. The card correctly sets `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)]))` and `keywords: vec![]`. PASS
- **Normal cast goes to graveyard**: When cast from hand, `in_graveyard` is `false` so `is_flashback` is `false` and `cast_with_flashback` is never set to `true`. `move_spell_after_resolve` therefore routes the spell to `Zone::Graveyard`. PASS
- **Mana cost correct**: `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)])` = {1}{U}. PASS
- **Flashback cost correct**: `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)])` = {2}{U}. PASS
- **Draw effect correct**: `on_resolve` calls `crate::engine::draw_cards(state, controller, 1)` — draws exactly one card for the spell's controller. PASS
- **"You may cast from your graveyard even without having been cast there"**: The engine simply checks `obj.zone == Zone::Graveyard` to offer flashback actions; it does not require a `cast_from_hand_first` flag. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flashback draws a card and exiles the spell: `flashback.rs:200` (`think_twice_draws_from_graveyard`)
- Flashback spell exiled after resolution (general): `flashback.rs:86` (`flashback_spell_is_exiled_after_resolve` using Geistflame)
- Flashback spell exiled when countered: `flashback.rs:129` (`flashback_spell_countered_is_exiled` using Geistflame)
- Flashback not offered without sufficient mana: `flashback.rs:65` (`flashback_not_offered_without_mana` using Geistflame)
- Flashback offered from graveyard when mana available: `flashback.rs:23` (`flashback_offered_from_graveyard` using Geistflame)
- Flashback not offered from hand: `flashback.rs:45` (`flashback_not_offered_from_hand` using Geistflame)
- Normal cast (from hand) goes to graveyard: `flashback.rs:110` (`normal_cast_goes_to_graveyard` using Geistflame); NOT TESTED directly for Think Twice
- Think Twice normal cast from hand goes to graveyard: NOT TESTED
- Timing restriction (instant-speed flashback): NOT TESTED explicitly
- Casting from graveyard without having been cast there first: NOT TESTED explicitly (covered implicitly by all graveyard setup helpers that place cards directly into graveyard)
