---
id: sharpened_pitchfork-01
status: new
card: Sharpened Pitchfork
audit_run_id: 2026-04-19-sharpened_pitchfork-audit
audit_model: sonnet
audit_tokens: 20752
audit_duration: 369
---

## Audit Finding

**Oracle text:**
> Equip {1}

**Code:**
> .filter(|o| can_be_targeted(state, o.id, controller, registry))

**Description:**
The equip ability is a targeted activated ability whose source is the Sharpened Pitchfork — an artifact. Per CR 702.16d, protection from artifacts prevents a creature from being targeted by sources with that quality. In `generate_ability_targets`, the `CreatureWithFilter` branch filters candidate targets via `can_be_targeted(state, o.id, controller, registry)`, which internally calls `can_be_targeted_by(state, target_id, caster, None, registry)` — passing `None` as the source object. The protection-from-source check inside `can_be_targeted_by` (lines 1463–1467 of engine.rs) is gated on `if let Some(sid) = source_id`, so it is unconditionally skipped for activated ability targets. As a result, a creature with 'protection from artifacts' on the battlefield is incorrectly presented as a valid equip target, and if the player selects it, `on_activate_ability` sets `attached_to` without any further legality check. The same defect applies to every other Equipment in the set, since they all share the same `CreatureWithFilter` target path in `generate_ability_targets`.

**Engine path:** mtg-engine/src/engine.rs:2020

**Required check:** 8c

**Affected cards:**
- Cobbled Wings
- Silver-Inlaid Dagger
- Butcher's Cleaver
- Wooden Stake
- Mask of Avacyn
- Inquisitor's Flail
- Runechanter's Pike
- Blazing Torch
- Demonmail Hauberk

## Tests

### equip_blocked_by_protection_from_artifacts
Scenario: A creature with protection from artifacts is on the battlefield; the Sharpened Pitchfork's equip ability must not offer that creature as a valid target.

