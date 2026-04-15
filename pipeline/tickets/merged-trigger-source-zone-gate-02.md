---
id: merged-trigger-source-zone-gate-02
status: merged
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: angel_of_flight_alabaster-02, charmbreaker_devils-01, geist_of_saint_traft-02, kessig_cagebreakers-01, splinterfright-02, undead_alchemist-01, gutter_grime-01, murder_of_crows-01, mentor_of_the_meek-03, trepanation_blade-01, merged-trigger-source-zone-gate-01
confirmed_at: 2026-04-15T06:19:56Z
test_run_id: 2026-04-14-merged-trigger-source-zone-gate-02-test
test_model: opus
test_tokens: 63083
test_duration: 1311
test_file: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs
tests_confirmed: 10
tests_total: 10
worktree: /Users/dlaw/mtg/.worktrees/fix-merged-trigger-source-zone-gate-02
fixed_at: 2026-04-15T06:29:58Z
fix_run_id: 2026-04-14-merged-trigger-source-zone-gate-02-fix
fix_model: opus
fix_tokens: 24342
fix_duration: 525
merged_at: 2026-04-15T06:34:24Z
---

# Trigger resolution skipped when source left battlefield (CR 113.7a, 603.10)

## Description
Per CR 113.7a, a triggered ability exists on the stack independently of its source — removal of the source does not affect the ability. The engine's trigger-resolution paths gate on the source permanent's current zone: UpkeepTrigger at triggers.rs:1306-1311, AttacksTrigger at triggers.rs:1332-1337, and watcher variants (DeathWatch, EnterWatch) at triggers.rs:1266-1268. Card-level handlers compound the problem by independently gating on `obj.zone == Zone::Battlefield`. For effects that do not reference the source (create a token, return a card, mill, draw/discard, etc.), this incorrectly fizzles the entire ability when the source is removed in response.

## Engine path
- triggers.rs:1306-1311 (UpkeepTrigger resolution zone gate)
- triggers.rs:1332-1337 (AttacksTrigger resolution zone gate)
- triggers.rs:1266-1268 (EnterWatch/DeathWatch resolution zone gate)
- triggers.rs:1320-1351 (watcher-variant trigger resolutions with the same gate)

## Tests

### test_angel_of_flight_alabaster_trigger_resolves_after_death
Source ticket: angel_of_flight_alabaster-02
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_angel_of_flight_alabaster_trigger_resolves_after_death
Scenario: Angel of Flight Alabaster's upkeep trigger goes on the stack targeting a Spirit in graveyard. In response, destroy the Angel. Verify the trigger resolves and the Spirit returns to hand.

### test_charmbreaker_devils_trigger_resolves_after_death
Source ticket: charmbreaker_devils-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_charmbreaker_devils_trigger_resolves_after_death
Scenario: Charmbreaker Devils' upkeep trigger goes on the stack with an instant in graveyard. Destroy the Devils in response. Verify the trigger resolves and the instant returns to hand.

### test_geist_of_saint_traft_angel_token_created_after_death
Source ticket: geist_of_saint_traft-02
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_geist_of_saint_traft_angel_token_created_after_death
Scenario: Geist attacks (hexproof prevents most removal but e.g., sacrifice effect removes it). Destroy Geist in response to the attack trigger. Verify the Angel token is still created.

### test_kessig_cagebreakers_tokens_created_after_death
Source ticket: kessig_cagebreakers-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_kessig_cagebreakers_tokens_created_after_death
Scenario: Cagebreakers attacks with N creature cards in graveyard. Destroy Cagebreakers in response. Verify N+1 Wolf tokens are created (N original + Cagebreakers itself now in graveyard).

### test_splinterfright_mill_resolves_after_death
Source ticket: splinterfright-02
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_splinterfright_mill_resolves_after_death
Scenario: Splinterfright's upkeep trigger goes on the stack. Destroy Splinterfright in response. Verify the mill-2 effect still resolves.

### test_undead_alchemist_watch_resolves_after_death
Source ticket: undead_alchemist-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_undead_alchemist_watch_resolves_after_death
Scenario: Undead Alchemist triggers on a creature being milled. Destroy Undead Alchemist in response (e.g., via same combat step). Verify the creature card is exiled and a Zombie token created.

### test_gutter_grime_creates_token_when_ltb
Source ticket: gutter_grime-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_gutter_grime_creates_token_when_ltb
Scenario: Gutter Grime and a nontoken creature are destroyed simultaneously (e.g., Planar Cleansing). Verify the death trigger partially resolves: a 0/0 Ooze token is created (which dies to SBA). At minimum, the token-creation instruction should not be skipped.

### test_murder_of_crows_trigger_resolves_after_simultaneous_death
Source ticket: murder_of_crows-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_murder_of_crows_trigger_resolves_after_simultaneous_death
Scenario: Murder of Crows and another creature die simultaneously (e.g., board wipe). The DeathWatch trigger is created via the simultaneously_dead list. Verify the trigger resolves: the controller is presented with the YesNo draw choice, even though Murder of Crows is in the graveyard.

### test_mentor_of_the_meek_trigger_resolves_after_removal
Source ticket: mentor_of_the_meek-03
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_mentor_of_the_meek_trigger_resolves_after_removal
Scenario: Mentor of the Meek is on the battlefield. A 1/1 creature enters, creating an EnterWatch trigger. Before the trigger resolves, destroy Mentor. Verify the trigger still resolves: the controller is presented with the YesNo pay choice per CR 113.7a.

### test_trepanation_blade_trigger_resolves_after_equipment_destroyed
Source ticket: trepanation_blade-01
Implementation: mtg-engine/tests/pipeline_bugs_merged_trigger_source_zone_gate_02.rs::test_trepanation_blade_trigger_resolves_after_equipment_destroyed
Scenario: Trepanation Blade is equipped to a creature. The creature attacks, putting the Blade's AttacksTrigger on the stack. Before the trigger resolves, destroy the Blade. Verify the trigger still resolves: the defender's library is milled and the creature receives +N/+0.

## Also closes

- merged-trigger-source-zone-gate-01

## Test Run Results

- **test_angel_of_flight_alabaster_trigger_resolves_after_death** — confirmed
  - test fn: `test_angel_of_flight_alabaster_trigger_resolves_after_death`
  - assertion: CR 113.7a: Angel upkeep trigger should return Spirit to hand even after Angel is destroyed
- **test_charmbreaker_devils_trigger_resolves_after_death** — confirmed
  - test fn: `test_charmbreaker_devils_trigger_resolves_after_death`
  - assertion: CR 113.7a: Devils upkeep trigger should return instant to hand even after Devils are destroyed
- **test_geist_of_saint_traft_angel_token_created_after_death** — confirmed
  - test fn: `test_geist_of_saint_traft_angel_token_created_after_death`
  - assertion: CR 113.7a: Geist attack trigger should create Angel token even after Geist is destroyed
- **test_kessig_cagebreakers_tokens_created_after_death** — confirmed
  - test fn: `test_kessig_cagebreakers_tokens_created_after_death`
  - assertion: CR 113.7a: Cagebreakers should create 3 Wolf tokens even after death (2 original + itself)
- **test_splinterfright_mill_resolves_after_death** — confirmed
  - test fn: `test_splinterfright_mill_resolves_after_death`
  - assertion: CR 113.7a: Splinterfright upkeep trigger should mill 2 even after destruction
- **test_undead_alchemist_watch_resolves_after_death** — confirmed
  - test fn: `test_undead_alchemist_watch_resolves_after_death`
  - assertion: CR 113.7a: Alchemist trigger should exile milled creature even after Alchemist is destroyed
- **test_gutter_grime_creates_token_when_ltb** — confirmed
  - test fn: `test_gutter_grime_creates_token_when_ltb`
  - assertion: CR 603.10: Gutter Grime death-watch should create Ooze token after simultaneous destruction
- **test_murder_of_crows_trigger_resolves_after_simultaneous_death** — confirmed
  - test fn: `test_murder_of_crows_trigger_resolves_after_simultaneous_death`
  - assertion: CR 603.10: Murder of Crows trigger should present draw choice even after simultaneous death
- **test_mentor_of_the_meek_trigger_resolves_after_removal** — confirmed
  - test fn: `test_mentor_of_the_meek_trigger_resolves_after_removal`
  - assertion: CR 113.7a: Mentor trigger should present pay choice even after Mentor is destroyed
- **test_trepanation_blade_trigger_resolves_after_equipment_destroyed** — confirmed
  - test fn: `test_trepanation_blade_trigger_resolves_after_equipment_destroyed`
  - assertion: CR 113.7a: Trepanation Blade trigger should mill even after equipment is destroyed

## Fix Result

status: fixed
files_changed: - mtg-engine/src/triggers.rs
- mtg-engine/src/state.rs
- mtg-engine/src/cards/isd/angel_of_flight_alabaster.rs
- mtg-engine/src/cards/isd/charmbreaker_devils.rs
- mtg-engine/src/cards/isd/splinterfright.rs
- mtg-engine/src/cards/isd/geist_of_saint_traft.rs
- mtg-engine/src/cards/isd/kessig_cagebreakers.rs
- mtg-engine/src/cards/isd/trepanation_blade.rs
- mtg-engine/src/cards/isd/mentor_of_the_meek.rs
- mtg-engine/src/cards/isd/gutter_grime.rs
- mtg-engine/src/cards/isd/murder_of_crows.rs
- mtg-engine/src/cards/isd/undead_alchemist.rs

Per CR 113.7a, a triggered ability on the stack exists independently of its source — removing the source does not counter the ability. The engine had zone gates at two layers that incorrectly fizzled triggers when the source left the battlefield:

1. **Engine-level gates in triggers.rs**: UpkeepTrigger, AttacksTrigger, EnterWatch, and CreatureCardMilledWatch all checked `source.zone == Battlefield` before dispatching to card handlers. These gates were removed since the trigger was already validated when placed on the stack.

2. **Card-level gates**: Nine card handlers independently matched `o.zone == Zone::Battlefield` to extract the controller, returning early if the source had moved. Changed all nine to extract the controller regardless of zone (the object still exists, just in a different zone).

3. **Equipment last-known-info**: When equipment leaves the battlefield, `attached_to` is cleared. Added `last_attached_to` preservation in `card_state` (state.rs) so the Trepanation Blade handler can find the equipped creature via last known information. Also added a defender fallback using `state.opponent(controller)` when combat state is unavailable.
