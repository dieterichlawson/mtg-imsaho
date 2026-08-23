---
id: butcher_s_cleaver-01
status: fixed
card: Butcher's Cleaver
audit_run_id: 2026-04-19-butcher_s_cleaver-audit
audit_model: sonnet
audit_tokens: 32807
audit_duration: 713
fixed_sha: eb10c286f92469fd334d00ead56072b930dce6eb
fixed_at: 2026-08-23T23:03:53Z
test_file: mtg-engine/tests/curse_and_equip_scope.rs
fix_note: cluster fix: equip target generation no longer excludes the already-attached creature (CR 702.6a)
---

## Audit Finding

**Oracle text:**
> Equip {3}

**Code:**
> let already_attached: Option<ObjectId> = state.get_object(source_id)
    .filter(|o| o.is_equipment)
    .and_then(|o| o.attached_to);
state.all_objects_in_zone(Zone::Battlefield).iter()
    ...
    .filter(|o| already_attached != Some(o.id))

**Description:**
The `generate_ability_targets` function for `TargetRequirement::CreatureWithFilter` unconditionally excludes the creature currently attached to the equipment from the valid target list. Per CR 702.6a, the equip ability reads 'Attach this permanent to target creature you control' with no restriction against targeting the already-attached creature. This makes it impossible to activate the equip ability targeting the same host — for example, when a player wants to re-equip after a triggered effect has provisionally detached it, or simply to spend mana while the only valid creature is already wearing the equipment. Butcher's Cleaver uses `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` for its equip ability, so the filter silently suppresses that action.

**Engine path:** mtg-engine/src/engine.rs:2022

**Required check:** 8c

**Affected cards:**
- Blazing Torch
- Cobbled Wings
- Demonmail Hauberk
- Inquisitor's Flail
- Mask of Avacyn
- Runechanter's Pike
- Sharpened Pitchfork
- Silver Inlaid Dagger
- Trepanation Blade
- Wooden Stake

## Tests

### reequip_to_same_creature_blocked
Scenario: Butcher's Cleaver is attached to the only creature the player controls; player tries to activate Equip {3} — the action should list that creature as a valid target, but the engine's already_attached filter incorrectly removes it, leaving no valid targets and suppressing the action entirely.

