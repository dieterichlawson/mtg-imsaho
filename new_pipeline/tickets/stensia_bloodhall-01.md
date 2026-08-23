---
id: stensia_bloodhall-01
status: fixed
card: Stensia Bloodhall
audit_run_id: 2026-04-19-stensia_bloodhall-audit
audit_model: sonnet
audit_tokens: 29778
audit_duration: 562
fixed_sha: c15d59216468a939ae6b78cb28062bbb8d811628
fixed_at: 2026-08-23T16:52:36Z
test_file: mtg-engine/tests/ability_target_protection.rs
fix_note: cluster fix: generate_ability_targets now threads Some(source_id) into can_be_targeted_by (CR 702.16b)
---

## Audit Finding

**Oracle text:**
> {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker. [Ruling] Like other lands, Stensia Bloodhall is colorless. The damage it deals is from a colorless source, even though activating its ability requires colored mana.

**Code:**
> if is_pw && can_be_targeted(state, obj.id, controller, registry) {
    let t = Target::Object(obj.id);
    if behavior.is_valid_target(state, controller, &t, registry) {
        targets.push(t);
    }
}

// can_be_targeted is defined as:
fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
    can_be_targeted_by(state, target_id, caster, None, registry)
}

**Description:**
In the `PlayerOrPlaneswalker` branch of `generate_ability_targets` (engine.rs:2046), planeswalker candidates are filtered by `can_be_targeted(state, obj.id, controller, registry)`. That helper (engine.rs:1447–1448) passes `None` as the `source_id` argument to `can_be_targeted_by`, so the protection-from-source check at engine.rs:1463–1466 is never reached. Per the ruling, Stensia Bloodhall is a colorless source even though its activation requires {B}{R}. If a planeswalker had any protection quality matching the Bloodhall (e.g., protection from colorless via a `TemporaryEffect::GrantProtection`), it would be incorrectly listed as a valid target. The `apply_pending_effect` handler for `Target::Object` does call `has_protection_from` at resolution (engine.rs:3457), so the damage itself would be prevented — but the ability would have been illegally declared targeting that planeswalker, violating CR 115.4 and 702.16.

**Engine path:** mtg-engine/src/engine.rs:2046

**Required check:** 8f

**Affected cards:**
- Stensia Bloodhall

## Tests

### planeswalker_with_protection_not_valid_target
Scenario: A planeswalker on the battlefield that has been granted until-EOT protection from a quality matching the Bloodhall should not appear in the legal target list for the activated ability.

