# Audit: Darkthicket Wolf

## Scryfall Reference
- **Name:** Darkthicket Wolf
- **Cost:** {1}{G}
- **Type:** Creature -- Wolf
- **Oracle:** {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
- **P/T:** 2/2
- **Keywords:** none

## Implementation: `darkthicket_wolf.rs`
- **Name:** Darkthicket Wolf -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Wolf"] -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** none -- CORRECT
- **Activated ability:** {2}{G}: +2/+2 until end of turn -- CORRECT
- **Once per turn:** true -- CORRECT

## Issues
None

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.

### Card Data
| Field       | Oracle             | Implementation          | Status  |
|-------------|--------------------|-------------------------|---------|
| Name        | Darkthicket Wolf   | "Darkthicket Wolf"      | CORRECT |
| Mana cost   | {1}{G}             | Generic(1), Green       | CORRECT |
| Type        | Creature -- Wolf   | Creature, ["Wolf"]      | CORRECT |
| P/T         | 2/2                | 2/2                     | CORRECT |
| Keywords    | (none)             | []                      | CORRECT |

### Activated Ability
| Field            | Oracle                        | Implementation                    | Status  |
|------------------|-------------------------------|-----------------------------------|---------|
| Cost             | {2}{G}                        | Generic(2), Green                 | CORRECT |
| Effect           | +2/+2 until end of turn       | power_mod: 2, toughness_mod: 2    | CORRECT |
| Once per turn    | Activate only once each turn  | once_per_turn: true               | CORRECT |
| Requires tap     | (no tap symbol)               | requires_tap: false               | CORRECT |
| Zone restriction | (implicit: battlefield)       | Zone::Battlefield check           | CORRECT |

### Oracle Text String
- **Oracle (Scryfall):** "{2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn."
- **Implementation oracle_text:** "{2}{G}: Darkthicket Wolf gets +2/+2 until end of turn. Activate only once each turn."
- **Note:** Scryfall uses "This creature" (modern template); implementation uses card name. Functionally equivalent; cosmetic only.

### Tests
- `darkthicket_wolf_has_correct_stats` -- PASS
- `darkthicket_wolf_gets_plus_2_plus_2` -- PASS
- `darkthicket_wolf_once_per_turn` -- PASS

### Verdict
**PASS** -- Implementation is fully correct. One cosmetic note: the `oracle_text` field uses the card's name instead of "This creature" per the current Scryfall template, but this has no mechanical impact.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues
Minor oracle text mismatch: code uses `"{2}{G}: Darkthicket Wolf gets +2/+2 until end of turn. Activate only once each turn."` but current oracle uses `"This creature"` template instead of the card name. Behavior is fully correct: activated ability costs {2}{G}, grants +2/+2 via UntilEndOfTurnEffect, once_per_turn is true, does not require tap. P/T 2/2 and cost {1}{G} match. Subtypes ["Wolf"] match.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:50

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Type line**: Creature — Wolf
**Status**: PASS

### Code issues
None. All card data fields match oracle exactly:
- Name: "Darkthicket Wolf" -- correct
- Mana cost: {1}{G} (Generic(1), Green) -- correct
- Type: Creature with subtypes ["Wolf"] -- correct
- P/T: 2/2 -- correct
- Oracle text string: matches Scryfall verbatim (uses "This creature" template)
- Activated ability: cost {2}{G} (Generic(2), Green), effect +2/+2 via UntilEndOfTurnEffect, once_per_turn: true, requires_tap: false, zone check: Battlefield only -- all correct
- No keywords, no flashback, no triggered abilities -- correct

Engine-level note: `abilities_activated_this_turn` is never cleared at end of turn or untap step in `engine.rs`. This would cause the once-per-turn restriction to persist across turns, affecting all cards with `once_per_turn: true`. This is an engine bug, not a Darkthicket Wolf implementation bug.

### Tricky interactions checked (min 3)
1. **Once-per-turn enforcement**: Engine checks `activated_this_turn.contains(&ab.ability_index)` at line 358 of engine.rs before allowing activation. The ability_index (0) is inserted into the set at line 1778 after activation. Correctly prevents second activation in the same turn. Test `darkthicket_wolf_once_per_turn` confirms.
2. **Effect stacking with multiple creatures**: The UntilEndOfTurnEffect targets a specific ObjectId, so pumping one Darkthicket Wolf does not affect another. The effect is additive in the `effective_power`/`effective_toughness` calculations.
3. **End-of-turn cleanup**: UntilEndOfTurnEffect is cleared at cleanup step (engine.rs line 3021: `state.until_end_of_turn_effects.clear()`), so the +2/+2 correctly expires at end of turn.
4. **Instant-speed activation**: `sorcery_speed_only: false` allows activation during combat or on opponent's turn, which is correct for this ability (no sorcery-speed restriction in oracle text).

### Test coverage
- `darkthicket_wolf_has_correct_stats` -- verifies P/T 2/2 and Wolf subtype
- `darkthicket_wolf_gets_plus_2_plus_2` -- verifies activation produces 4/4 effective stats
- `darkthicket_wolf_once_per_turn` -- verifies second activation is blocked within the same turn
- All 3 tests PASS

## Re-evaluation — 2026-04-02 21:10

**Status**: ISSUE (reclassified from PASS)

### Code issues
- `abilities_activated_this_turn` is never cleared between turns in `engine.rs`, so the once-per-turn restriction on the {2}{G} pump ability permanently locks after first use -- the ability can only ever be activated once per game instead of once per turn
