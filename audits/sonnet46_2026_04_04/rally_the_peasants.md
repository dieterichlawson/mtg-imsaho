## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures you control get +2/+0 until end of turn.
Flashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Snapshot vs. continuous effect**: The card stores specific ObjectIds in `until_end_of_turn_effects` rather than using a continuous `ContinuousEffect::ModifyPT`. This correctly means only creatures on the battlefield at resolution are affected. Creatures entering later are not buffed. Matches the ruling "Only creatures you control when Rally the Peasants resolves will be affected." — PASS
- **"Creatures you control" scope**: Code filters `obj.controller == controller` where controller is fetched from the spell object itself on the stack. Correctly identifies the caster's creatures only; opponent's creatures (tested via `ready_creature(&mut state, P1, 3, 3)`) are unaffected. — PASS
- **"Until end of turn" cleanup**: `state.until_end_of_turn_effects.clear()` is called in the cleanup step (`engine.rs:3021`). The effect is properly transient. — PASS
- **Flashback cost and exile**: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Red)]))` matches `{2}{R}`. `state.move_spell_after_resolve(object_id)` is called, which exiles the spell if `cast_with_flashback` is true, else sends it to graveyard. — PASS
- **Flashback countered = still exiled**: The `cast_with_flashback` flag is set at cast time, so if countered the spell still goes to exile via the fizzle/counter path in the engine. Verified by generic flashback test `flashback_spell_countered_is_exiled` in `flashback.rs`. — PASS
- **Creature detection via `power.is_some()`**: The check `obj.power.is_some()` is the standard engine-wide proxy for "is a creature." Non-creature permanents (enchantments, artifacts, planeswalkers, lands) have `power: None` in this engine. This is consistent with dozens of other cards and engine code. — PASS
- **Mana cost correctness**: `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::White)])` correctly represents `{2}{W}`. — PASS
- **+2/+0 not +2/+2**: `power_mod: 2, toughness_mod: 0` — toughness is unaffected as the oracle text specifies. — PASS
- **No targets required**: `target_requirement()` defaults to `TargetRequirement::None` (no override in the card). Correct, as the spell has no targets. — PASS
- **Card registration**: Registered in `cards/mod.rs:548` and declared in `isd/mod.rs:175`. — PASS
- **Instant timing**: `card_types: vec![CardType::Instant]` — the engine uses this to allow casting at instant speed. — PASS
- **Flashback keyword not in `keywords` vec**: Flashback is handled via the separate `flashback_cost` field; the engine's `Keyword` enum does not include Flashback. `keywords: vec![]` is correct. — PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic +2/+0 buff to all your creatures, opponent unaffected: `innistrad_cards.rs:158` (`rally_the_peasants_buffs_all_your_creatures`) — TESTED
- Toughness unchanged: `innistrad_cards.rs:173` (asserts `effective_toughness(c1) == Some(2)`) — TESTED
- Only creatures controlled at resolution are affected (snapshot, not continuous): NOT TESTED (the test casts from hand with no late-entering creatures)
- Flashback exile after resolution: NOT TESTED specifically for Rally the Peasants (generic flashback exile tested for Geistflame and Bump in the Night in `flashback.rs`)
- Flashback exile after countering: NOT TESTED for Rally the Peasants
- Effect expiring at end of turn (cleanup step): NOT TESTED
- Normal cast goes to graveyard, not exile: NOT TESTED for Rally the Peasants
