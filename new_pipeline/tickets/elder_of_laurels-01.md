---
id: elder_of_laurels-01
status: new
card: Elder of Laurels
audit_run_id: 2026-04-19-elder_of_laurels-audit
audit_model: sonnet
audit_tokens: 18493
audit_duration: 356
---

## Audit Finding

**Oracle text:**
> {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.

**Code:**
> .filter(|o| can_be_targeted(state, o.id, controller, registry))

**Description:**
The `generate_ability_targets` function at engine.rs:2008 uses `can_be_targeted` (which calls `can_be_targeted_by` with `source_id: None`) for the `TargetRequirement::Creature` branch. The protection-from-source check inside `can_be_targeted_by` (engine.rs:1462–1466) only fires when `source_id` is `Some(sid)`. Elder of Laurels is a green permanent; its ability is therefore a green source. Per CR 702.16, a creature with protection from green cannot be the target of abilities from green sources. Additionally, the Elder is a Human Advisor creature, so creatures with protection from Humans, protection from Advisors, or protection from creatures are equally shielded. Because `source_id` is `None`, none of these protection checks run, and such creatures are incorrectly offered as legal targets and can be illegally buffed. The fix is to replace the `can_be_targeted` call in `generate_ability_targets` with `can_be_targeted_by(state, o.id, controller, Some(source_id), registry)` so the protection-from-source path is exercised for all targeted activated abilities.

**Engine path:** mtg-engine/src/engine.rs:2008

**Required check:** 8f

**Affected cards:**
- Kessig Wolf Run
- Blazing Torch
- Stensia Bloodhall
- Avacynian Priest
- Olivia Voldaren

## Tests

### elder_protection_from_green_not_targetable
Scenario: A creature with protection from green is on the battlefield; Elder of Laurels' activated ability should not list it as a valid target, but the engine incorrectly offers it

### elder_protection_from_creatures_not_targetable
Scenario: A creature with protection from creatures is on the battlefield; Elder of Laurels' activated ability should not list it as a valid target, but the engine incorrectly offers it

