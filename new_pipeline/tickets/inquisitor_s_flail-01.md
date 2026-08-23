---
id: inquisitor_s_flail-01
status: fixed
card: Inquisitor's Flail
audit_run_id: 2026-04-19-inquisitor_s_flail-audit
audit_model: sonnet
audit_tokens: 31812
audit_duration: 691
fixed_sha: c15d59216468a939ae6b78cb28062bbb8d811628
fixed_at: 2026-08-23T16:52:36Z
test_file: mtg-engine/tests/ability_target_protection.rs
fix_note: cluster fix: generate_ability_targets now threads Some(source_id) into can_be_targeted_by (CR 702.16b)
---

## Audit Finding

**Oracle text:**
> Equip {2}

**Code:**
> .filter(|o| can_be_targeted(state, o.id, controller, registry))

**Description:**
In `generate_ability_targets` (engine.rs:2020), the `CreatureWithFilter` branch — used by the Equip {2} ability — calls `can_be_targeted(state, o.id, controller, registry)`. That wrapper is defined at engine.rs:1447–1448 as `can_be_targeted_by(state, target_id, caster, None, registry)`, passing `None` as the source object. Inside `can_be_targeted_by`, the protection-from-source check is gated on `if let Some(sid) = source_id { … }` (lines 1463–1467), so when source_id is None the check is entirely skipped. Per CR 702.16b, protection from an Equipment's quality (e.g., "Protection from Artifacts") prevents the protected creature from being equipped by that Equipment. Because the Flail is a colorless Artifact, any creature with Protection from Artifacts or Protection from Colorless cannot legally be equipped, yet the engine will offer such creatures as valid equip targets and permit the equip to succeed.

**Engine path:** mtg-engine/src/engine.rs:2020

**Required check:** 8f

**Affected cards:**
- Cobbled Wings
- Mask of Avacyn
- Blazing Torch

## Tests

### flail_equip_cannot_target_protection_from_artifacts
Scenario: Attempt to equip Inquisitor's Flail to a creature with Protection from Artifacts — it should not appear as a valid equip target and the equip action should be unavailable.

