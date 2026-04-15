---
id: merged-target-as-choose-02
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: bitterheart_witch-01, bloodgift_demon-01, slayer_of_the_wicked-01, merged-target-as-choose-01
---

# Targeted triggered abilities implemented with target_requirement: None (CR 603.3c/d)

## Description
Oracle text containing "target" means per CR 603.3c/d that the target must be chosen when the ability is put on the stack — not during resolution. Several triggered abilities are implemented with `target_requirement: None` in their `TriggeredAbilityDef`, causing the trigger to go on the stack untargeted (triggers.rs:1163-1166) and deferring target selection to the resolution handler. This bypasses CR 603.3c (no legal target means no stack), CR 608.2b (resolution-time legality re-check), and CR 115 protections (hexproof, protection, shroud). The engine already supports targeted triggers via `TargetRequirement` variants.

## Engine path
- triggers.rs:1163-1166 (untargeted trigger pushed directly to stack)
- triggers.rs:1169-1218 (targeted trigger infrastructure — what should be used)

## Tests

### test_bitterheart_witch_uses_target_player_semantics
Source ticket: bitterheart_witch-01
Implementation: (not yet written)
Scenario: Bitterheart Witch dies with two opponents in the game. Verify the target player is chosen when the trigger is put on the stack (visible in the stack entry), and if that player gains shroud/hexproof before resolution, the trigger fizzles.

### test_bloodgift_demon_uses_target_player_semantics
Source ticket: bloodgift_demon-01
Implementation: (not yet written)
Scenario: Bloodgift Demon's upkeep triggers with multiple opponents. Verify the target player is chosen when the trigger is put on the stack and fizzles if that player gains hexproof before resolution.

### test_slayer_of_the_wicked_hexproof_zombie_not_targetable
Source ticket: slayer_of_the_wicked-01
Implementation: (not yet written)
Scenario: Place a Zombie token with hexproof on the opponent's battlefield. Enter Slayer of the Wicked. Verify the hexproof Zombie does NOT appear in the ETB trigger's target options and, if no other valid targets exist, the trigger does not go on the stack (CR 603.3c).

### test_slayer_of_the_wicked_no_trigger_without_valid_targets
Source ticket: slayer_of_the_wicked-01
Implementation: (not yet written)
Scenario: Enter Slayer of the Wicked when no Vampire, Werewolf, or Zombie exists on the battlefield. Verify no ETB trigger is placed on the stack. Currently the trigger goes on the stack because target_requirement: None bypasses the CR 603.3c check.

## Also closes

- merged-target-as-choose-01

