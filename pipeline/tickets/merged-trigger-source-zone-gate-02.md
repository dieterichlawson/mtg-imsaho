---
id: merged-trigger-source-zone-gate-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: angel_of_flight_alabaster-02, charmbreaker_devils-01, geist_of_saint_traft-02, kessig_cagebreakers-01, splinterfright-02, undead_alchemist-01, gutter_grime-01, murder_of_crows-01, mentor_of_the_meek-03, trepanation_blade-01, merged-trigger-source-zone-gate-01
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
Implementation: (not yet written)
Scenario: Angel of Flight Alabaster's upkeep trigger goes on the stack targeting a Spirit in graveyard. In response, destroy the Angel. Verify the trigger resolves and the Spirit returns to hand.

### test_charmbreaker_devils_trigger_resolves_after_death
Source ticket: charmbreaker_devils-01
Implementation: (not yet written)
Scenario: Charmbreaker Devils' upkeep trigger goes on the stack with an instant in graveyard. Destroy the Devils in response. Verify the trigger resolves and the instant returns to hand.

### test_geist_of_saint_traft_angel_token_created_after_death
Source ticket: geist_of_saint_traft-02
Implementation: (not yet written)
Scenario: Geist attacks (hexproof prevents most removal but e.g., sacrifice effect removes it). Destroy Geist in response to the attack trigger. Verify the Angel token is still created.

### test_kessig_cagebreakers_tokens_created_after_death
Source ticket: kessig_cagebreakers-01
Implementation: (not yet written)
Scenario: Cagebreakers attacks with N creature cards in graveyard. Destroy Cagebreakers in response. Verify N+1 Wolf tokens are created (N original + Cagebreakers itself now in graveyard).

### test_splinterfright_mill_resolves_after_death
Source ticket: splinterfright-02
Implementation: (not yet written)
Scenario: Splinterfright's upkeep trigger goes on the stack. Destroy Splinterfright in response. Verify the mill-2 effect still resolves.

### test_undead_alchemist_watch_resolves_after_death
Source ticket: undead_alchemist-01
Implementation: (not yet written)
Scenario: Undead Alchemist triggers on a creature being milled. Destroy Undead Alchemist in response (e.g., via same combat step). Verify the creature card is exiled and a Zombie token created.

### test_gutter_grime_creates_token_when_ltb
Source ticket: gutter_grime-01
Implementation: (not yet written)
Scenario: Gutter Grime and a nontoken creature are destroyed simultaneously (e.g., Planar Cleansing). Verify the death trigger partially resolves: a 0/0 Ooze token is created (which dies to SBA). At minimum, the token-creation instruction should not be skipped.

### test_murder_of_crows_trigger_resolves_after_simultaneous_death
Source ticket: murder_of_crows-01
Implementation: (not yet written)
Scenario: Murder of Crows and another creature die simultaneously (e.g., board wipe). The DeathWatch trigger is created via the simultaneously_dead list. Verify the trigger resolves: the controller is presented with the YesNo draw choice, even though Murder of Crows is in the graveyard.

### test_mentor_of_the_meek_trigger_resolves_after_removal
Source ticket: mentor_of_the_meek-03
Implementation: (not yet written)
Scenario: Mentor of the Meek is on the battlefield. A 1/1 creature enters, creating an EnterWatch trigger. Before the trigger resolves, destroy Mentor. Verify the trigger still resolves: the controller is presented with the YesNo pay choice per CR 113.7a.

### test_trepanation_blade_trigger_resolves_after_equipment_destroyed
Source ticket: trepanation_blade-01
Implementation: (not yet written)
Scenario: Trepanation Blade is equipped to a creature. The creature attacks, putting the Blade's AttacksTrigger on the stack. Before the trigger resolves, destroy the Blade. Verify the trigger still resolves: the defender's library is milled and the creature receives +N/+0.

## Also closes

- merged-trigger-source-zone-gate-01

