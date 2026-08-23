---
id: kessig_wolf_run-01
status: fixed
card: Kessig Wolf Run
audit_run_id: 2026-04-19-kessig_wolf_run-audit
audit_model: sonnet
audit_tokens: 26628
audit_duration: 498
fixed_sha: c15d59216468a939ae6b78cb28062bbb8d811628
fixed_at: 2026-08-23T16:52:36Z
test_file: mtg-engine/tests/ability_target_protection.rs
fix_note: cluster fix: generate_ability_targets now threads Some(source_id) into can_be_targeted_by (CR 702.16b)
---

## Audit Finding

**Oracle text:**
> {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.

**Code:**
> TargetRequirement::Creature => {
    state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| o.power.is_some())
        .filter(|o| can_be_targeted(state, o.id, controller, registry))
        .map(|o| Target::Object(o.id))
        .filter(|t| behavior.is_valid_target(state, controller, t, registry))
        .collect()
}

// and:
fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
The activated ability target list is built by calling `can_be_targeted`, which is a thin wrapper that hard-codes `None` as the `source_id` argument to `can_be_targeted_by`. Because `source_id` is `None`, the protection-from-source branch inside `can_be_targeted_by` is never reached (guarded by `if let Some(sid) = source_id`). Kessig Wolf Run is a colorless Land with no subtypes, so the qualities that would ordinarily matter are: protection from colorless permanents, protection from lands, and protection from all sources. A creature with any of these protections cannot legally be targeted by Kessig Wolf Run's ability (CR 702.16b), but the engine will include it in the target list and allow the ability to resolve against it.

**Engine path:** mtg-engine/src/engine.rs:2005

**Required check:** 8f

**Affected cards:**
- all cards with targeted activated abilities

## Tests

### kessig_wolf_run_cannot_target_protection_from_colorless
Scenario: Player activates Kessig Wolf Run targeting a creature with protection from colorless permanents; the ability should be illegal and the creature should not appear in the target list.

