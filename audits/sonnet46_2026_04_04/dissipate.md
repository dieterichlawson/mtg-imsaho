## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Target spell exiled directly (not via graveyard first): pass — `state.move_object(*target_id, Zone::Exile)` is called directly; there is no intermediate `Zone::Graveyard` step, consistent with ruling [2004-10-04].
- "If countered this way" conditional (no exile if spell can't be countered): pass for current engine state — the engine has no "can't be countered" mechanic (`CantBeCountered` keyword, `cant_be_countered` flag, or related `ContinuousEffect` variant do not exist in the codebase). No spell in the engine has that property, so the conditional clause cannot currently be triggered incorrectly.
- Dissipate itself goes to graveyard, not exile: pass — `state.move_spell_after_resolve(object_id)` at the end of `on_resolve` moves Dissipate to `Zone::Graveyard` (since `cast_with_flashback` is false), confirmed by test assertion at `tier2_spells.rs:84`.
- Target no longer on stack at resolution (fizzle): pass — `stack.rs:80-86` checks target legality before calling `on_resolve`; if the target left the stack, Dissipate fizzles and moves to graveyard via `move_spell_after_resolve` without executing the exile logic.
- Triggered abilities on the stack cannot be targeted: pass — target generation for `TargetRequirement::Spell` in `engine.rs:1058-1063` uses `stack.iter().filter_map(|e| e.as_spell())`, which skips `StackEntry::Trigger` entries; `is_valid_target` also rejects any id not found in `state.objects` as a zone-Stack object.
- Dissipate cannot target itself: pass — target generation filters with `.filter(|&id| id != spell_id)` at `engine.rs:1062`.
- Mana cost {1}{U}{U}: pass — `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Blue)])` at `dissipate.rs:14-17`.
- Oracle text field matches Scryfall verbatim: pass — `dissipate.rs:24` contains the exact Scryfall oracle text.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Countered spell goes to exile (not graveyard): `tier2_spells.rs:82-83` — TESTED
- Dissipate itself goes to graveyard after resolving: `tier2_spells.rs:84` — TESTED
- Spell not countered because it can't be countered (no exile): NOT TESTED (engine does not implement "can't be countered")
- Fizzle case — target leaves stack before Dissipate resolves: NOT TESTED
