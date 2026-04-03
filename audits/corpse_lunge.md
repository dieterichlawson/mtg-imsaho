# Audit: Corpse Lunge

## Scryfall Reference
- **Name:** Corpse Lunge
- **Cost:** {2}{B}
- **Type:** Instant
- **Oracle:** As an additional cost to cast this spell, exile a creature card from your graveyard. Corpse Lunge deals damage equal to the exiled card's power to target creature.
- **P/T:** N/A
- **Keywords:** none

## Implementation: `corpse_lunge.rs`
- **Name:** Corpse Lunge -- CORRECT
- **Cost:** {2}{B} -- CORRECT
- **Type:** Instant -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Additional cost:** ExileCreaturesFromGraveyard(1) -- CORRECT
- **Target:** TargetRequirement::Creature -- CORRECT
- **Damage:** Uses NonCombatDamageDealt event -- CORRECT
- **Behavior:** Exiles creature from graveyard, deals damage equal to power to target creature -- CORRECT

## Issues

### Issue 1: Missing `damaged_by` tracking (Bug)

**Oracle text:** "Corpse Lunge deals damage equal to the exiled card's power to target creature."

Non-combat damage must update `damaged_by` on the target so that triggers tracking damage sources (e.g. Falkenrath Noble, deathtouch interactions) function correctly. All other non-combat damage cards in the codebase do this.

**Code in `corpse_lunge.rs` lines 50-53:**
```rust
if let Some(obj) = state.get_object_mut(*target_id) {
    if obj.zone == Zone::Battlefield {
        obj.damage_marked += damage;
        let name = obj.name.clone();
```

**Missing line after `obj.damage_marked += damage;`:**
```rust
obj.damaged_by.push(object_id);
```

**Compare `harvest_pyre.rs` lines 49-50:**
```rust
obj.damage_marked += count;
obj.damaged_by.push(object_id);
```

**Compare `blasphemous_act.rs` lines 54-55:**
```rust
obj.damage_marked += 13;
obj.damaged_by.push(object_id);
```

### Issue 2: Engine auto-selects highest-power creature instead of player choice (Engine-Level)

**Oracle text:** "As an additional cost to cast this spell, exile a creature card from your graveyard."

The player should choose which creature card to exile. The engine code in `engine.rs` line 1399 sorts by highest power first and auto-selects:
```rust
exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first
```

This is an engine-level simplification affecting all `ExileCreaturesFromGraveyard` cards (Skaab Ruinator, Stitched Drake, etc.), not specific to `corpse_lunge.rs`. However, it is most impactful for Corpse Lunge since the exiled card's power directly determines the damage dealt, meaning a player might strategically want to exile a lower-power creature to preserve a higher-power one for later reanimation.

The test `corpse_lunge_picks_highest_power_creature` at line 504 of `tier8_cards.rs` enshrines this auto-pick behavior, which does not match the oracle text's player-choice semantics.

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Type line**: Instant
**Status**: PASS

### Code issues

All previously identified issues have been fixed:

1. **`damaged_by` tracking (prior Issue 1): FIXED.** Line 53 now includes `obj.damaged_by.push(object_id);`, matching the pattern used by other damage-dealing cards (e.g., `harvest_pyre.rs`, `blasphemous_act.rs`).

2. **Card data correct.** Cost `{2}{B}` (lines 16-18), type Instant (line 20), additional cost `ExileCreaturesFromGraveyard(1)` (line 29), target `TargetRequirement::Creature` (line 35).

3. **Damage delivery correct.** Reads stored `exiled_power` from card_state (lines 40-43), deals that as damage via `damage_marked` (line 52), emits `NonCombatDamageDealt` event (lines 55-58).

4. **Engine-level auto-selection of exile target (prior Issue 2): Unchanged.** This is an engine-level limitation, not specific to this card file.

### Tricky interactions checked
- Zero or negative power on exiled creature: `power.max(0)` on line 47 correctly prevents negative damage.
- Target creature leaving battlefield before resolution: checked via `obj.zone == Zone::Battlefield` on line 51.
- Damage event correctly emitted for downstream triggers.

### Test coverage
- `corpse_lunge_picks_highest_power_creature` -- validates damage equals exiled creature's power.
- No test for zero-power exiled creature edge case.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:45

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/93/corpse-lunge)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Type line**: Instant
**Status**: PASS

### Code issues

1. **Oracle text field cosmetic mismatch (non-behavioral):** The `oracle_text` field in `card_data()` says `"As an additional cost to cast Corpse Lunge"` but the official Scryfall oracle text says `"As an additional cost to cast this spell"`. This does not affect game behavior.

2. **`if damage > 0` guard (line 47):** The code skips the entire damage-dealing block when power is 0. In MTG, "deals 0 damage" is technically distinct from "no damage event" (relevant for triggers like "whenever a source deals damage"). Practically inconsequential in the current engine.

3. **Engine-level auto-selection of exile target:** Unchanged from prior audits. The engine auto-picks the highest-power creature in graveyard rather than allowing player choice. This is an engine-wide limitation for `ExileCreaturesFromGraveyard`, not card-specific.

No behavioral bugs found. All card data (name, mana cost `{2}{B}`, type Instant, `ExileCreaturesFromGraveyard(1)`, `TargetRequirement::Creature`) is correct. Damage delivery reads stored `exiled_power` from card_state, marks damage, tracks `damaged_by`, emits `NonCombatDamageDealt`, and calls `move_spell_after_resolve`.

### Tricky interactions checked (min 3)

1. **Zero-power exiled creature:** `power.max(0) as u32` on line 47 prevents negative damage. With zero power, the `if damage > 0` guard prevents any damage event from being emitted.
2. **Target creature leaves battlefield before resolution:** Checked via `obj.zone == Zone::Battlefield` on line 51. If the target is no longer on the battlefield, no damage is dealt.
3. **Exiled card's power used, not current power:** The power is stored at cast time in `card_state["exiled_power"]` (engine.rs lines 1589-1592) and read at resolution (lines 40-43). This correctly uses the power the creature had when it was exiled as an additional cost, matching the "last known information" principle from the rulings.
4. **Flashback interaction:** `move_spell_after_resolve` correctly handles flashback (exiles instead of going to graveyard).

### Test coverage

- `corpse_lunge_deals_damage_equal_to_exiled_power` -- 4/4 in graveyard, deals 4 damage to target creature. Verifies exiled creature moves to exile zone.
- `corpse_lunge_no_graveyard_creature_deals_no_damage` -- No graveyard creature available, verifies 0 damage dealt.
- `corpse_lunge_picks_highest_power_creature` -- Two creatures (2/2 and 5/5) in graveyard, verifies highest-power (5/5) is exiled and 5 damage dealt.
- All 3 tests pass.
