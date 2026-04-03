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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:58

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/14/feeling-of-dread)
**Oracle text**: Tap up to two target creatures.
Flashback {1}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS (minor LLM card knowledge issue noted below)

### Code issues
- LLM card knowledge in `mtg-player/src/llm.rs` line 112 says "Tap target creature." (singular) but oracle text is "Tap up to two target creatures." This does not affect gameplay (the engine uses `UpToTargets(2, Creature)` correctly), only the AI player's description of the card.

### Tricky interactions checked (min 3)
1. **Partial target illegality (rule 608.2b)**: If one of two targets becomes illegal before resolution, the surviving target is still tapped. Confirmed by independent loop in `on_resolve` and by test `multi_target_spell_with_one_target_dying`.
2. **Full fizzle**: If both targets become illegal, the spell is countered by game rules. Confirmed by `is_target_legal` check in `stack.rs` and test `multi_target_spell_with_all_targets_dying`.
3. **Flashback exile**: When cast via flashback, `move_spell_after_resolve` checks `cast_with_flashback` flag and exiles instead of sending to graveyard. Confirmed by test `flashback_spell_is_exiled_after_resolve`.
4. **Already-tapped creatures**: Setting `tapped = true` on an already-tapped creature is legal and harmless, matching MTG rules (tapping doesn't require untapped state for the spell's effect).

### Test coverage
1. `feeling_of_dread_taps_creature` (flashback.rs:433) — taps a single target creature
2. `feeling_of_dread_taps_two` (card_mechanics.rs:553) — taps two target creatures
3. `multi_target_spell_with_one_target_dying` (spell_fizzle.rs:233) — partial target illegality, surviving target still tapped
4. `multi_target_spell_with_all_targets_dying` (spell_fizzle.rs:265) — all targets illegal, spell fizzles
