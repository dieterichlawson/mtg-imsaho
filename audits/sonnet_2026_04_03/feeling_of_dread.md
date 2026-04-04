## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues
- Oracle text field is incomplete (mtg-engine/src/cards/isd/feeling_of_dread.rs:23)
  - Oracle text says: `Tap up to two target creatures.\nFlashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)`
  - Code does: `"Tap up to two target creatures.".into()` (missing flashback reminder text)

### Tricky interactions checked
- "Up to two" targeting allows 0-2 target selection: pass (UpToTargets(2, Creature) correctly implemented)
- Partial resolution when one target becomes illegal: pass (based on test in spell_fizzle.rs:259)
- Flashback cost payment ({1}{U} vs normal {1}{W}): pass (flashback_cost field correctly set)
- Flashback spell exile after resolution: pass (move_spell_after_resolve handles cast_with_flashback flag)
- Targeting validation (only battlefield creatures): pass (on_resolve checks obj.zone == Zone::Battlefield)
- Normal cast goes to graveyard: pass (move_spell_after_resolve handles both modes)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic tapping functionality: `mtg-engine/tests/flashback.rs:433` (feeling_of_dread_taps_creature)
- Two-target resolution: `mtg-engine/tests/card_mechanics.rs:553` (feeling_of_dread_taps_two) 
- Partial resolution with illegal target: `mtg-engine/tests/spell_fizzle.rs:233` (multi_target_spell_with_one_target_dying)
- Complete fizzle with all targets illegal: `mtg-engine/tests/spell_fizzle.rs:264` (multi_target_spell_both_targets_dying)
- Flashback casting and exile behavior: NOT TESTED
- Flashback cost vs normal cost: NOT TESTED
- Up to zero targets (casting with no targets): NOT TESTED