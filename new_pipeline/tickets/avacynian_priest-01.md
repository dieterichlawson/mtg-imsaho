---
id: avacynian_priest-01
status: fixed
card: Avacynian Priest
audit_run_id: 2026-04-19-avacynian_priest-audit
audit_model: sonnet
audit_tokens: 27351
audit_duration: 475
fixed_sha: c15d59216468a939ae6b78cb28062bbb8d811628
fixed_at: 2026-08-23T16:52:36Z
test_file: mtg-engine/tests/ability_target_protection.rs
fix_note: cluster fix: generate_ability_targets now threads Some(source_id) into can_be_targeted_by (CR 702.16b)
---

## Audit Finding

**Oracle text:**
> {1}, {T}: Tap target non-Human creature.

**Code:**
> TargetRequirement::Creature => {
    state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| o.power.is_some())
        .filter(|o| can_be_targeted(state, o.id, controller, registry))
        .map(|o| Target::Object(o.id))
        .filter(|t| behavior.is_valid_target(state, controller, t, registry))
        .collect()
}

// can_be_targeted (engine.rs:1447):
fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
The `TargetRequirement::Creature` branch in `generate_ability_targets` calls `can_be_targeted`, which is a thin wrapper that always passes `None` for `source_id` to `can_be_targeted_by`. The protection-from-source check inside `can_be_targeted_by` only runs when `source_id` is `Some`; with `None` it is silently skipped. As a result, creatures with protection from white, protection from Humans, or protection from Clerics — all qualities of Avacynian Priest — are included in the generated target list for the `{1}, {T}: Tap target non-Human creature` ability. Per CR 702.16b, a creature with protection from a quality cannot be targeted by sources with that quality. The spell-targeting path (`valid_targets_for_req`) correctly passes `Some(spell_id)` to `can_be_targeted_by`; the activated-ability path does not thread the source through to the protection check.

**Engine path:** mtg-engine/src/engine.rs:2008

**Required check:** 8f

**Affected cards:**
- Elder of Laurels
- Olivia Voldaren
- Skirsdag Cultist
- Wooden Stake
- Blazing Torch
- Stensia Bloodhall
- Daybreak Ranger
- Graveyard Shovel
- Mindshrieker
- Mikaeus, the Lunarch
- Disciple of Griselbrand
- Selfless Cathar

## Tests

### avacynian_priest_cannot_target_protection_from_white
Scenario: A non-Human creature with protection from white should not appear as a legal target for Avacynian Priest's {1},{T} ability, but it is incorrectly included in the target list because generate_ability_targets passes None for source_id.

