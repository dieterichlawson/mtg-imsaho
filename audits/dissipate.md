# Audit: Dissipate

## Reference (Scryfall)
- **Name:** Dissipate
- **Cost:** {1}{U}{U}
- **Type:** Instant
- **Oracle:** Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{U}{U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Target requirement: CORRECT (Spell)
- Counters spell: CORRECT (removes from stack)
- Exiles instead of graveyard: CORRECT (moves to Zone::Exile)
- P/T: CORRECT (N/A)

## Issues
None found.

## Audit (2026-04-02)

### Oracle Text (Scryfall)
Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.

### Card Data
- **Name:** CORRECT — `"Dissipate"`
- **Mana cost:** CORRECT — `Generic(1), Colored(Blue), Colored(Blue)` = {1}{U}{U}
- **Type:** CORRECT — `Instant`
- **Oracle text:** CORRECT — matches Scryfall verbatim
- **P/T:** CORRECT — N/A (none set)

### Counter Mechanic
- **Target requirement:** CORRECT — `TargetRequirement::Spell`
- **Target validation:** CORRECT — checks `obj.zone == Zone::Stack`
- **Stack removal:** CORRECT — `state.stack.retain(|e| e.as_spell() != Some(*target_id))` removes the countered spell from the stack

### Exile Instead of Graveyard
- **Countered spell destination:** CORRECT — `state.move_object(*target_id, Zone::Exile)` sends the countered spell to exile, not graveyard. This differs from vanilla Counterspell which calls `move_spell_after_resolve(*target_id)` (sends to graveyard).
- **Test confirms:** `dissipate_counters_and_exiles` in `mtg-engine/tests/tier2_spells.rs` asserts the countered spell ends up in `Zone::Exile`.

### Dissipate Self-Cleanup
- **move_spell_after_resolve(object_id):** CORRECT — Dissipate itself goes to graveyard (or exile if cast via flashback) after resolving.
- **Test confirms:** test asserts `state.get_object(diss).unwrap().zone == Zone::Graveyard`.

### Notes
- The engine does not yet model "can't be countered" spells. Per ruling (2004-10-04): "If the spell is not countered (because the spell it targets can't be countered), then it does not get exiled." This is a general engine limitation, not a Dissipate-specific bug.

### Verdict
PASS — no issues found.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:54
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/53/dissipate)
**Oracle text**: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
**Type line**: Instant
**Status**: PASS

### Code issues
None. Implementation correctly exiles the countered spell via `move_object(*target_id, Zone::Exile)` and sends Dissipate itself to graveyard via `move_spell_after_resolve(object_id)`. Oracle text in `card_data()` matches Scryfall verbatim. Card data (name, cost {1}{U}{U}, type Instant) all correct.

### Tricky interactions checked (min 3)
1. **Fizzle (target leaves stack before resolution)**: Handled by the framework fizzle check in `stack.rs` (lines 74-86), which verifies target legality before calling `on_resolve`. If the target spell is no longer on the stack, Dissipate fizzles and never enters `on_resolve`. Correct.
2. **Flashback spell countered by Dissipate**: The countered spell goes to exile via `move_object(*target_id, Zone::Exile)`, which is correct regardless of whether the target had flashback. (Flashback spells would go to exile anyway via `move_spell_after_resolve`, but Dissipate bypasses that and directly exiles.)
3. **"Can't be countered" spells**: The engine does not yet implement this mechanic, so no current bug exists. However, per ruling (2004-10-04), if the target can't be countered, it should not be exiled. The current implementation unconditionally exiles after checking `Zone::Stack`, which would need updating when "can't be countered" is added. This is a general engine limitation, not a Dissipate-specific defect.
4. **Dissipate itself cast with flashback**: Dissipate calls `move_spell_after_resolve(object_id)` for itself, which correctly checks `cast_with_flashback` and would exile Dissipate if it were cast via flashback. (Dissipate has no flashback cost, so this is defensive but correct.)

### Test coverage
- `dissipate_counters_and_exiles` in `mtg-engine/tests/tier2_spells.rs`: Casts a creature spell, counters it with Dissipate, asserts the creature is in `Zone::Exile` and Dissipate is in `Zone::Graveyard`. Covers the core mechanic.
- No test for fizzle case (target removed before resolution), but this is covered by the framework-level fizzle tests.
