---
id: cobbled_wings-01
status: fixed
card: Cobbled Wings
audit_run_id: 2026-04-19-cobbled_wings-audit
audit_model: sonnet
audit_tokens: 15194
audit_duration: 314
fixed_sha: eb10c286f92469fd334d00ead56072b930dce6eb
fixed_at: 2026-08-23T23:03:53Z
test_file: mtg-engine/tests/curse_and_equip_scope.rs
fix_note: cluster fix: equip target generation no longer excludes the already-attached creature (CR 702.6a)
---

## Audit Finding

**Oracle text:**
> Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)

**Code:**
> .filter(|o| already_attached != Some(o.id))

**Description:**
The `CreatureWithFilter` branch in `generate_ability_targets` (engine.rs:2022) excludes the currently-attached creature from the equip ability's legal targets. Per CR 702.6a, equip says "Attach this permanent to target creature you control" with no restriction on targeting the creature the equipment is already attached to. The filter is a UX shortcut (re-equipping to the same host is usually pointless) but is rules-incorrect: a player may legally re-equip to the same creature — for instance, to use a separate sacrifice cost trigger without changing which creature has flying. With Cobbled Wings attached to a player's only creature, the equip ability generates zero valid targets and cannot be activated at all, despite being legally payable.

**Engine path:** mtg-engine/src/engine.rs:2022

**Required check:** 8c

**Affected cards:**
- Trepanation Blade
- Wooden Stake
- Silver-Inlaid Dagger
- Sharpened Pitchfork
- Butcher's Cleaver
- Runechanter's Pike
- Inquisitor's Flail
- Demonmail Hauberk
- Blazing Torch
- Mask of Avacyn

## Tests

### cobbled_wings_equip_can_retarget_attached_creature
Scenario: Cobbled Wings is attached to the controller's only creature; the equip ability should offer that creature as a valid target (re-equip), but currently offers zero targets.

