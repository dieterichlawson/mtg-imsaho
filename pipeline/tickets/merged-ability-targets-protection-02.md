---
id: merged-ability-targets-protection-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: daybreak_ranger-03, kessig_wolf_run-02, stensia_bloodhall-02, merged-ability-targets-protection-01
---

# Activated-ability target enumeration skips protection-from check

## Description
`generate_ability_targets` (engine.rs:1988-2018) receives `source_id: ObjectId` as a parameter but calls `can_be_targeted(obj)` — the no-source variant — instead of `can_be_targeted_by(state, obj.id, controller, Some(source_id), registry)`. The protection-from-source check at engine.rs:1452-1465 is only performed when `source_id` is `Some`; with `None` the protection check is skipped. A permanent with protection from a source quality (color, type, etc.) can be illegally chosen as a target for any activated ability.

## Engine path
- engine.rs:1988-2018 (generate_ability_targets — calls can_be_targeted without source)
- engine.rs:2033-2051 (PlayerOrPlaneswalker branch — same issue)
- engine.rs:1445-1447 (can_be_targeted passes None)
- engine.rs:1452-1465 (can_be_targeted_by skips protection when source_id is None)

## Tests

### test_daybreak_ranger_ability_cannot_target_protection_from_green
Source ticket: daybreak_ranger-03
Implementation: (not yet written)
Scenario: Opponent controls a flying creature with protection from green (e.g., via Apostle's Blessing). Activate Daybreak Ranger's {T} ability. Verify that creature is NOT offered as a legal target.

### test_kessig_wolf_run_cannot_target_protection_from_green
Source ticket: kessig_wolf_run-02
Implementation: (not yet written)
Scenario: Opponent controls a creature with protection from green. Activate Kessig Wolf Run. Verify that creature is NOT offered as a legal target.

### test_stensia_bloodhall_cannot_target_protected_planeswalker
Source ticket: stensia_bloodhall-02
Implementation: (not yet written)
Scenario: Place Stensia Bloodhall on the battlefield controlled by P0. Place a planeswalker controlled by P1 with protection from colorless. Give P0 sufficient mana ({3}{B}{R}). Call legal_actions and assert that no ActivateAbility action for Bloodhall targets the protected planeswalker.

## Also closes

- merged-ability-targets-protection-01

