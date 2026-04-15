---
id: daybreak_ranger-03
status: closed-duplicate
card: Daybreak Ranger
card_file: mtg-engine/src/cards/isd/daybreak_ranger.rs
created: 2026-04-14T21:22:02Z
audit_run_id: 2026-04-14-daybreak_ranger-audit
audit_model: opus
audit_tokens: 12078
audit_duration: 368
duplicate_of: merged-ability-targets-protection-01
---

## Audit Finding

**Oracle text:**
> {T}: This creature deals 2 damage to target creature with flying.

**Code:**
> `generate_ability_targets` at engine.rs:2018: `.filter(|o| can_be_targeted(state, o.id, controller, registry))` — uses `can_be_targeted` which calls `can_be_targeted_by` with `source_id = None` (engine.rs:1446).

**Description:**
The activated ability target enumeration function `generate_ability_targets` (engine.rs:1988) receives `source_id: ObjectId` as a parameter but does not pass it to the targeting check. It calls `can_be_targeted` (no source) instead of `can_be_targeted_by(state, o.id, controller, Some(source_id), registry)`. The `can_be_targeted_by` function checks protection from source at line 1461–1464, but this check is skipped when `source_id` is `None`. This means a creature with flying that also has protection from green (e.g., via Apostle's Blessing) can be illegally targeted by Daybreak Ranger's ability. By contrast, the spell targeting path `valid_targets_for_req` (engine.rs:1737) correctly passes `Some(spell_id)` at line 1761.

**Engine path:**
- engine.rs:2018 (`can_be_targeted` call without source_id)
- engine.rs:1445–1446 (`can_be_targeted` → `can_be_targeted_by` with `None`)
- engine.rs:1452–1466 (`can_be_targeted_by` skips protection check when source_id is None)

**Required check:** 8f

**Affected cards:**
- Daybreak Ranger (front face)
- Nightfall Predator (back face fight ability)
- ALL cards with targeted activated abilities (engine-wide)
