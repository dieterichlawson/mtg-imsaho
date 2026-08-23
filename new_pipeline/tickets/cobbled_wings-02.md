---
id: cobbled_wings-02
status: new
card: Cobbled Wings
audit_run_id: 2026-04-19-cobbled_wings-audit
audit_model: sonnet
audit_tokens: 15194
audit_duration: 314
---

## Audit Finding

**Oracle text:**
> Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)

**Code:**
> fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
The `can_be_targeted` wrapper called at engine.rs:2020 (inside the `CreatureWithFilter` target-enumeration branch) passes `source_id: None` to `can_be_targeted_by`, skipping the protection-from-source check in `has_protection_from`. Per CR 115 and 702.16, a creature with protection from artifacts cannot be targeted by abilities from artifact sources. Cobbled Wings is a colorless artifact, so its equip ability is an ability of an artifact. A creature with "protection from artifacts" should be excluded from the equip's target list, but the engine presents it as a legal target because `source_id = None` bypasses the `has_protection_from(target_id, source_id)` check.

**Engine path:** mtg-engine/src/engine.rs:1447

**Required check:** 8f

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

### cobbled_wings_equip_cannot_target_protection_from_artifacts
Scenario: Controller has a creature with protection from artifacts and another without; the equip ability should exclude the protected creature as an illegal target, but currently includes it.

