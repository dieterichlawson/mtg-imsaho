## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Up to two" allows zero targets**: `generate_cast_actions_with_targets` iterates `for k in 0..=(*max).min(options.len())` starting from 0, so casting with 0 targets is legal. On resolve the loop body is a no-op. Correct — pass.
- **Partial illegal targets (one of two dies before resolution)**: `stack.rs` `resolve_spell` only fizzles when `!targets.iter().any(|t| is_target_legal(...))` — i.e., ALL targets illegal. If one is still on the battlefield the spell resolves. `on_resolve` then checks `obj.zone == Zone::Battlefield` before tapping each target, so the dead creature is skipped and the live creature is tapped. Matches the 2011-09-22 ruling — pass.
- **All targets illegal — fizzle**: Both targets removed before resolution → `any_legal` is false → spell is countered by game rules, `on_resolve` never called. `move_spell_after_resolve` still runs, so a flashback-cast fizzle goes to Exile correctly — pass.
- **Flashback cost is {1}{U} not {1}{W}**: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)]))` — correct blue cost — pass.
- **Flashback spell exiled on resolution**: `move_spell_after_resolve` checks `obj.cast_with_flashback`; if true moves to `Zone::Exile`, else `Zone::Graveyard`. Engine sets `cast_with_flashback = true` when casting from graveyard via flashback (`engine.rs:1636-1637`). Correct — pass.
- **Flashback spell exiled if countered**: `stack.rs` `resolve_spell` calls `state.move_spell_after_resolve(object_id)` in the fizzle path as well. `cast_with_flashback` is already set on the object, so a countered flashback spell also goes to Exile. Correct per the ruling "always exiled afterward" — pass.
- **Instant timing for flashback**: Engine's graveyard cast loop checks `is_instant || has_flash` for instant-speed permission. Feeling of Dread is `CardType::Instant`, so `is_instant = true` — flashback can be used at instant speed. Correct — pass.
- **Any creature can be targeted (no controller restriction)**: `TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature))` — `Creature` resolves to all creatures on the battlefield regardless of controller. Correct, the oracle text places no controller restriction — pass.
- **Targeting already-tapped creature**: Setting `obj.tapped = true` on an already-tapped creature is a no-op. Mechanically fine; oracle text says "Tap" (not "untap" or conditional) — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic tap effect (single target): `flashback.rs:433` (`feeling_of_dread_taps_creature`) — TESTED
- Two-target simultaneous tap (both valid): NOT TESTED
- Flashback cast of Feeling of Dread specifically (exile after flashback use): NOT TESTED (only generic flashback exile tests using Geistflame/Bump in the Night exist)
- Partial illegal target — one of two dies before resolution: `spell_fizzle.rs:233` (`multi_target_spell_with_one_target_dying`) — TESTED
- All targets illegal — fizzle: `spell_fizzle.rs:264` (`multi_target_spell_with_all_targets_dying`) — TESTED
- Fizzled flashback spell goes to exile: `fizzle.rs:137` (`flashback_spell_fizzle_goes_to_exile`) — TESTED (uses Geistflame, not FoD, but covers the engine path)
- Countered flashback spell goes to exile: `flashback.rs:129` (`flashback_spell_countered_is_exiled`) — TESTED (uses Geistflame)
- Zero-target cast (casting with 0 of "up to two"): NOT TESTED
