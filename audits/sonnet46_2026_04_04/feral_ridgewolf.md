## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample
{1}{R}: This creature gets +2/+0 until end of turn.
**Type line**: Creature — Wolf
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {2}{R} matches code (`ManaSymbol::Generic(2)` + `ManaSymbol::Colored(Color::Red)`): pass
- P/T 1/2 matches code (`power: Some(1)`, `toughness: Some(2)`): pass
- Subtype "Wolf" matches code (`subtypes: vec!["Wolf".into()]`): pass
- Keyword Trample matches code (`keywords: vec![Keyword::Trample]`): pass
- Activated ability cost {1}{R} matches code (`ManaSymbol::Generic(1)` + `ManaSymbol::Colored(Color::Red)`): pass
- Effect is +2/+0 (power only): code uses `power_mod: 2, toughness_mod: 0` in `UntilEndOfTurnEffect`: pass
- Effect targets "This creature" (the activating object): code uses `target: object_id`, the wolf itself: pass
- "Until end of turn" expiry: `state.until_end_of_turn_effects.clear()` is called at `Step::Cleanup` in `engine.rs:3021`: pass
- No tap cost: `requires_tap: false` matches oracle (no {T} symbol): pass
- No once-per-turn restriction: `once_per_turn: false` matches oracle (no per-turn limit stated): pass
- No sorcery-speed restriction: `sorcery_speed_only: false` matches oracle (no timing restriction stated): pass
- Multiple activations stack: each activation pushes a new `UntilEndOfTurnEffect`; `effective_power` in `state.rs` sums all matching entries: pass
- Ability only available on battlefield: `activated_abilities` returns the vec only when `zone == Zone::Battlefield`: pass
- Effect does not modify toughness: `toughness_mod: 0` confirmed: pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card stats (P/T 1/2, Trample, Wolf subtype): `mtg-engine/tests/activated_abilities.rs:121` (`feral_ridgewolf_has_correct_stats`)
- Single activation yields +2/+0: `mtg-engine/tests/activated_abilities.rs:132` (`feral_ridgewolf_gets_plus_2_plus_0`)
- Multiple activations stack correctly: `mtg-engine/tests/activated_abilities.rs:156` (`feral_ridgewolf_can_activate_multiple_times`)
- Effect expires at end of turn (cleanup step): NOT TESTED
- Ability unavailable when not on battlefield: NOT TESTED
- Trample functioning correctly in combat: NOT TESTED
