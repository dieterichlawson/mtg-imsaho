---
id: merged-target-as-choose-01
status: closed-duplicate
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: bitterheart_witch-01, bloodgift_demon-01
duplicate_of: merged-target-as-choose-02
---

# "Target player" triggers implemented as `target_requirement: None` (CR 603.3c/d)

## Description
Oracle text "target player" means per CR 603.3c/d that the target must be chosen when the ability is put on the stack — not during resolution. Some triggered abilities are implemented with `target_requirement: None` in their `TriggeredAbilityDef`, causing the trigger to go on the stack untargeted (`triggers.rs:1163-1166`) and deferring player selection to the resolution handler. Consequences: (1) opponents receive priority without knowing the target, (2) the CR 603.3c rule (no legal target → no stack) is bypassed, (3) CR 608.2b resolution-time legality re-check does not apply, (4) the ability uses "choose" rather than "target" semantics, bypassing CR 115 protections. The engine already supports targeted triggers via `TargetRequirement::PlayerOnly` (see Falkenrath Noble at falkenrath_noble.rs:33 for a correct example).

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
