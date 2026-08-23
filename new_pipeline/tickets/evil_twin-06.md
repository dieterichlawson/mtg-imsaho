---
id: evil_twin-06
status: fixed
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
fixed_sha: c15d59216468a939ae6b78cb28062bbb8d811628
fixed_at: 2026-08-23T16:52:36Z
test_file: mtg-engine/tests/ability_target_protection.rs
fix_note: cluster fix: generate_ability_targets now threads Some(source_id) into can_be_targeted_by (CR 702.16b)
---

## Audit Finding

**Oracle text:**
> {U}{B}, {T}: Destroy target creature with the same name as this creature.

**Code:**
> .filter(|o| can_be_targeted(state, o.id, controller, registry))

**Description:**
In `generate_ability_targets` (engine.rs:2020), the Evil Twin destroy ability's candidate list is filtered by `can_be_targeted`, which calls `can_be_targeted_by` with `source_id = None`. This means protection from the activating source is never checked. The destroy ability is Blue and Black (costs {U}{B}). A creature with protection from Blue or protection from Black cannot legally be the target of a Blue or Black ability (702.16b: protection grants immunity to targeting by sources of the protected quality). However, the engine's omission of `source_id` in `can_be_targeted` means those creatures appear in the candidate list and can be selected as legal targets. The correct call is `can_be_targeted_by(state, o.id, controller, Some(source_id), registry)` as used on the spell-targeting path. This is an engine-wide pattern (documented in auditor-insights.md) that manifests here for Evil Twin's destroy ability.

**Engine path:** mtg-engine/src/engine.rs:2020

**Required check:** 8c

## Tests

### evil_twin_destroy_blocked_by_protection_from_blue
Scenario: Evil Twin copies a creature named 'X'; another creature named 'X' has protection from blue; activating the destroy ability should not be able to target the protection-from-blue creature, but currently it can.

