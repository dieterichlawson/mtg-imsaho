---
id: olivia_voldaren-01
status: new
card: Olivia Voldaren
audit_run_id: 2026-04-19-olivia_voldaren-audit
audit_model: sonnet
audit_tokens: 22278
audit_duration: 724
---

## Audit Finding

**Oracle text:**
> That creature becomes a Vampire in addition to its other types.

**Code:**
> if let Some(obj) = state.get_object_mut(*target_id) {
    if !obj.subtypes.contains(&"Vampire".to_string()) {
        obj.subtypes.push("Vampire".to_string());
    }
}

**Description:**
Ability 0 adds the Vampire subtype to a creature by pushing to obj.subtypes. The move_object cleanup block in state.rs clears tapped, damage_marked, counters, and attached_to when a permanent leaves the battlefield, but does NOT clear obj.subtypes for non-transformed objects. Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence. If the converted creature later dies and is reanimated (Graveyard → Battlefield), the same object data is reused and obj.subtypes still contains "Vampire", giving the reborn creature a Vampire subtype it should not have. The fix belongs in the move_object cleanup block, which should clear obj.subtypes on zone departure so the next zone entry starts clean.

**Engine path:** mtg-engine/src/cards/isd/olivia_voldaren.rs:112

**Required check:** 8a

**Affected cards:**
- Grimoire of the Dead

## Tests

### vampire_subtype_does_not_persist_through_death_and_reanimation
Scenario: Olivia converts a Grizzly Bears to a Vampire; the Bears die and are reanimated; verify the reanimated Bears are NOT a Vampire.

### vampire_subtype_does_not_persist_through_bounce_and_replay
Scenario: Olivia converts a creature to a Vampire; the creature is bounced to hand and replayed; verify it is NOT a Vampire when it re-enters.

