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
