---
id: merged-ability-targets-protection-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: daybreak_ranger-03, kessig_wolf_run-02
---

# Activated-ability target enumeration skips protection-from check

## Description
`generate_ability_targets` (engine.rs:1988-2018) receives `source_id: ObjectId` as a parameter but calls `can_be_targeted(obj)` — the no-source variant — instead of `can_be_targeted_by(state, obj.id, controller, Some(source_id), registry)`. The `can_be_targeted_by` function at engine.rs:1452-1465 only checks protection from source when `source_id` is `Some`; with `None` the protection check is skipped. Consequence: a creature with protection from red (or protection from a source quality) can be illegally chosen as a target by an activated ability. The spell-targeting path (`valid_targets_for_req` at engine.rs:1737) correctly passes `Some(spell_id)` at line 1761.

## Engine path
- engine.rs:1988-2018 (generate_ability_targets — calls can_be_targeted without source)
- engine.rs:1445-1447 (can_be_targeted passes None)
- engine.rs:1452-1465 (can_be_targeted_by skips protection when source_id is None)
- engine.rs:1737-1761 (valid_targets_for_req — correct reference implementation)

## Tests

### test_daybreak_ranger_ability_cannot_target_protection_from_green
Source ticket: daybreak_ranger-03
Implementation: (not yet written)
Scenario: Opponent controls a flying creature with protection from green (e.g., via Apostle's Blessing). Activate Daybreak Ranger's {T} ability. Verify that creature is NOT offered as a legal target.

### test_kessig_wolf_run_cannot_target_protection_from_green
Source ticket: kessig_wolf_run-02
Implementation: (not yet written)
Scenario: Opponent controls a creature with protection from green. Activate Kessig Wolf Run. Verify that creature is NOT offered as a legal target.

