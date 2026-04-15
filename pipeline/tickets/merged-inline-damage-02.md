---
id: merged-inline-damage-02
status: confirmed
card: multiple
created: 2026-04-15T04:40:54Z
kind: consolidated
source_tickets: balefire_dragon-01, olivia_voldaren-01, daybreak_ranger-01, devils_play-01, into_the_maw_of_hell-01, blasphemous_act-01, harvest_pyre-01, stensia_bloodhall-01, merged-inline-damage-01
confirmed_at: 2026-04-15T05:36:12Z
test_run_id: 2026-04-14-merged-inline-damage-02-test
test_model: opus
test_tokens: 60925
test_duration: 1457
test_file: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs
tests_confirmed: 11
tests_total: 11
worktree: /Users/dlaw/mtg/.worktrees/fix-merged-inline-damage-02
---

# Inline damage writes bypass the central damage handler (CR 702.16e, 614.1a)

## Description
Multiple cards apply damage by directly writing to `obj.damage_marked` or manipulating player life / planeswalker loyalty instead of routing through `apply_pending_effect(PendingEffect::DealDamage)` at engine.rs:3424. The central handler enforces damage prevention / replacement effects (engine.rs:3426-3447, including Unbreathing Horde's counter-remove-to-prevent), protection from source via `has_protection_from` (engine.rs:3449-3453, CR 702.16e), and planeswalker loyalty-counter removal (engine.rs:3460-3466, CR 120.3c). Inline writes skip all three checks, causing protection, prevention, and planeswalker targeting to be silently incorrect for any card using this pattern.

## Engine path
- engine.rs:3424-3478 (central PendingEffect::DealDamage handler — what all sources should use)
- engine.rs:3449-3453 (protection-from-source check bypassed by inlining)
- engine.rs:3426-3447 (PreventDamageRemoveCounter and other replacement effects bypassed)
- engine.rs:3460-3466 (planeswalker loyalty removal bypassed)
- helpers.rs:49-83 (resolve_damage helper that also inlines damage)

## Tests

### test_balefire_dragon_respects_protection_from_red
Source ticket: balefire_dragon-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::test_balefire_dragon_respects_protection_from_red
Scenario: Balefire Dragon deals combat damage to a player who controls a creature with protection from red. Verify that creature takes 0 damage from the triggered ability.

### test_olivia_voldaren_first_ability_respects_protection
Source ticket: olivia_voldaren-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::test_olivia_voldaren_first_ability_respects_protection
Scenario: Activate Olivia's {1}{R} ability targeting a creature with protection from black. Verify no damage is dealt, no Vampire subtype added, and no +1/+1 counter placed on Olivia.

### test_daybreak_ranger_ability_respects_protection
Source ticket: daybreak_ranger-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::test_daybreak_ranger_ability_respects_protection
Scenario: Activate Daybreak Ranger's {T} ability targeting a flying creature with protection from green. Verify no damage is dealt.

### test_devils_play_removes_planeswalker_loyalty
Source ticket: devils_play-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::test_devils_play_removes_planeswalker_loyalty
Scenario: Cast Devil's Play for X=3 targeting an opponent's planeswalker with 4 loyalty. Verify loyalty drops to 1 (removed by counter-removal) and the planeswalker's damage_marked is 0.

### test_into_the_maw_of_hell_respects_protection
Source ticket: into_the_maw_of_hell-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::test_into_the_maw_of_hell_respects_protection
Scenario: Cast Into the Maw of Hell targeting a land and a creature with protection from red. Verify the creature takes 0 damage.

### blasphemous_act_protection_from_red_prevents_damage
Source ticket: blasphemous_act-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::blasphemous_act_protection_from_red_prevents_damage
Scenario: Place a creature with protection from red and another without protection on the battlefield. Cast and resolve Blasphemous Act. Assert the protected creature has 0 damage marked, and the unprotected creature has 13 damage marked.

### blasphemous_act_damage_prevention_replacement_effect
Source ticket: blasphemous_act-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::blasphemous_act_damage_prevention_replacement_effect
Scenario: Place an Unbreathing Horde with +1/+1 counters on the battlefield alongside a vanilla creature. Cast and resolve Blasphemous Act. Assert that Unbreathing Horde lost a +1/+1 counter instead of taking damage (damage_marked remains 0), and the vanilla creature has 13 damage marked.

### harvest_pyre_inline_damage_bypasses_protection
Source ticket: harvest_pyre-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::harvest_pyre_inline_damage_bypasses_protection
Scenario: Set up a battlefield with a creature that has protection from red. Cast Harvest Pyre exiling cards from graveyard, targeting that creature. Assert the creature takes 0 damage.

### harvest_pyre_inline_damage_bypasses_prevention
Source ticket: harvest_pyre-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::harvest_pyre_inline_damage_bypasses_prevention
Scenario: Set up a battlefield with Unbreathing Horde with +1/+1 counters. Cast Harvest Pyre targeting Unbreathing Horde. Assert that damage is prevented and a +1/+1 counter is removed.

### bloodhall_damage_respects_protection
Source ticket: stensia_bloodhall-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::bloodhall_damage_respects_protection
Scenario: Place Stensia Bloodhall on the battlefield. Place a planeswalker with protection from the source. Activate Bloodhall's ability targeting the planeswalker. Assert the planeswalker's loyalty counters are unchanged because protection prevents the damage.

### bloodhall_damage_respects_prevention
Source ticket: stensia_bloodhall-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_inline_damage_02.rs::bloodhall_damage_respects_prevention
Scenario: Place Stensia Bloodhall on the battlefield. Place a planeswalker with a damage prevention shield. Activate Bloodhall targeting the planeswalker. Assert that damage is prevented and the prevention effect is consumed.

## Also closes

- merged-inline-damage-01

## Test Run Results

- **test_balefire_dragon_respects_protection_from_red** — confirmed
  - test fn: `test_balefire_dragon_respects_protection_from_red`
  - assertion: CR 702.16e: creature with protection from Dragons takes 0 damage from Balefire Dragon trigger (left: 6, right: 0)
- **test_olivia_voldaren_first_ability_respects_protection** — confirmed
  - test fn: `test_olivia_voldaren_first_ability_respects_protection`
  - assertion: CR 702.16e: creature with protection from Vampire takes no damage from Olivia (left: 1, right: 0)
- **test_daybreak_ranger_ability_respects_protection** — confirmed
  - test fn: `test_daybreak_ranger_ability_respects_protection`
  - assertion: CR 702.16e: creature with protection from Human takes no damage from Daybreak Ranger (left: 2, right: 0)
- **test_devils_play_removes_planeswalker_loyalty** — confirmed
  - test fn: `test_devils_play_removes_planeswalker_loyalty`
  - assertion: CR 120.3c: 3 damage to a 4-loyalty planeswalker should leave 1 loyalty counter (left: 4, right: 1)
- **test_into_the_maw_of_hell_respects_protection** — confirmed
  - test fn: `test_into_the_maw_of_hell_respects_protection`
  - assertion: CR 702.16e: creature with protection from red takes 0 damage from Into the Maw of Hell (left: 13, right: 0)
- **blasphemous_act_protection_from_red_prevents_damage** — confirmed
  - test fn: `blasphemous_act_protection_from_red_prevents_damage`
  - assertion: CR 702.16e: creature with protection from red takes 0 damage from Blasphemous Act (left: 13, right: 0)
- **blasphemous_act_damage_prevention_replacement_effect** — confirmed
  - test fn: `blasphemous_act_damage_prevention_replacement_effect`
  - assertion: CR 614.1a: Unbreathing Horde damage should be prevented by removing a +1/+1 counter (left: 13, right: 0)
- **harvest_pyre_inline_damage_bypasses_protection** — confirmed
  - test fn: `harvest_pyre_inline_damage_bypasses_protection`
  - assertion: CR 702.16e: creature with protection from red takes 0 damage from Harvest Pyre (left: 3, right: 0)
- **harvest_pyre_inline_damage_bypasses_prevention** — confirmed
  - test fn: `harvest_pyre_inline_damage_bypasses_prevention`
  - assertion: CR 614.1a: Unbreathing Horde damage should be prevented by removing a counter (left: 3, right: 0)
- **bloodhall_damage_respects_protection** — confirmed
  - test fn: `bloodhall_damage_respects_protection`
  - assertion: CR 702.16e: protection prevents Stensia Bloodhall damage, loyalty should be unchanged (left: 2, right: 4)
- **bloodhall_damage_respects_prevention** — confirmed
  - test fn: `bloodhall_damage_respects_prevention`
  - assertion: CR 614.1a: damage prevented by counter removal, loyalty should be unchanged (left: 2, right: 4)
