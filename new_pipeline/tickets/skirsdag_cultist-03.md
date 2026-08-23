---
id: skirsdag_cultist-03
status: new
card: Skirsdag Cultist
audit_run_id: 2026-04-19-skirsdag_cultist-audit
audit_model: sonnet
audit_tokens: 29132
audit_duration: 1795
---

## Audit Finding

**Oracle text:**
> This creature deals 2 damage to any target.

**Code:**
> fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
All activated ability target enumeration goes through can_be_targeted (engine.rs:1447-1448), which calls can_be_targeted_by with source_id: None. The protection-from-source guard inside can_be_targeted_by (line 1463-1465) only executes when source_id is Some(sid). With None, has_protection_from is never called for activated ability targets. Skirsdag Cultist is a Red permanent; a creature with protection from Red should be an illegal target (CR 115.4, 702.16), but it appears in the generated target list because the source ObjectId is never threaded through to the protection check. The spell-targeting path (valid_targets_for_req) correctly passes Some(spell_id) to can_be_targeted_by; only the activated-ability path has this gap. Note that even after fixing target enumeration, the inline damage path (Finding 1) would still need a separate fix to respect protection at resolution time.

**Engine path:** mtg-engine/src/engine.rs:1447

**Required check:** 8f

## Tests

### skirsdag_cultist_can_illegally_target_protected_creature
Scenario: A creature with protection from Red is on the battlefield; Skirsdag Cultist's activated ability should not present that creature as a valid target, but the protected creature appears in the action list because source_id is not passed to the protection check.

