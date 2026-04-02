# Audit: Feeling of Dread

## Reference (Scryfall)
- **Name:** Feeling of Dread
- **Cost:** {1}{W}
- **Type:** Instant
- **Oracle:** Tap up to two target creatures. Flashback {1}{U}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{W})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Flashback cost: CORRECT ({1}{U})
- Target requirement: CORRECT (UpToTargets(2, Creature))
- Taps targets: CORRECT (sets obj.tapped = true)
- P/T: CORRECT (N/A)

## Issues
None found.

---

## Audit (2026-04-02)

### Oracle Text (Scryfall)
- **Name:** Feeling of Dread
- **Mana Cost:** {1}{W}
- **Type:** Instant
- **Oracle Text:** Tap up to two target creatures.
- **Flashback:** {1}{U}
- **Keywords:** Flashback

### Implementation File
`mtg-engine/src/cards/isd/feeling_of_dread.rs`

### Audit Checklist

#### Card Data
- **Name:** "Feeling of Dread" — correct.
- **Mana cost:** `{1}{W}` (`Generic(1), Colored(White)`) — correct.
- **Card type:** `Instant` — correct.
- **Oracle text field:** "Tap up to two target creatures." — correct.
- **Flashback cost:** `{1}{U}` (`Generic(1), Colored(Blue)`) — correct.
- **Keywords vec:** Empty. Consistent with codebase convention; flashback is implemented via `flashback_cost: Some(...)`.

#### Targeting
- `TargetRequirement::UpToTargets(2, Box::new(TargetRequirement::Creature))` — correct for "up to two target creatures."

#### on_resolve
- Iterates all targets; for each `Target::Object`, checks the object is on the battlefield and sets `tapped = true` — correct.
- If one target becomes illegal, the other is still tapped (loop processes each independently) — matches ruling from 2011-09-22.

#### move_spell_after_resolve
- Called at end of `on_resolve` — correct.
- `move_spell_after_resolve` checks `cast_with_flashback` flag: exiles if flashback, otherwise moves to graveyard — correct flashback behavior.

### Tests
1. `feeling_of_dread_taps_creature` (`mtg-engine/tests/flashback.rs:433`) — verifies tapping a single creature.
2. `feeling_of_dread_taps_two` (`mtg-engine/tests/card_mechanics.rs:553`) — verifies tapping two creatures.

### Verdict
**PASS** — No issues found. Implementation matches oracle text exactly.
