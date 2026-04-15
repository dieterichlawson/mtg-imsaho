---
id: merged-trigger-target-recheck-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: angel_of_flight_alabaster-03, grimgrin_corpse_born-01, morkrut_banshee-01, snapcaster_mage-02
---

# Triggered ability targets not re-checked at resolution (CR 608.2b)

## Description
Per CR 608.2b, when a spell or ability resolves, each of its targets is checked against legality criteria. Illegal targets cause that target to be ignored; if all targets become illegal, the ability is removed from the stack and has no effect. The engine's stack-based resolver for spells does re-check (stack.rs:87-108), but the triggered-ability resolver (`triggers.rs:1232-1249`, particularly EnteredBattlefield at 1243-1249) dispatches handlers without any target-legality re-check. A target that became illegal (left its zone, gained hexproof, gained protection, etc.) between the trigger going on the stack and resolving is still passed to the handler, producing stale effects or silent no-ops.

## Engine path
- triggers.rs:1232-1249 (trigger resolution dispatch — no target check)
- stack.rs:87-108 (spell resolution fizzle check — reference implementation)

## Tests

### test_angel_of_flight_alabaster_fizzles_on_illegal_target
Source ticket: angel_of_flight_alabaster-03
Implementation: (not yet written)
Scenario: Upkeep trigger targeting a Spirit in graveyard. Exile that Spirit card in response (e.g., Purify the Grave). Verify the trigger fizzles — the Spirit is not moved from exile to hand.

### test_grimgrin_attack_trigger_fizzles_on_illegal_target
Source ticket: grimgrin_corpse_born-01
Implementation: (not yet written)
Scenario: Grimgrin attacks targeting a creature with the destroy trigger. Give the target hexproof in response. Verify the trigger fizzles: no destruction AND no +1/+1 counter added to Grimgrin.

### test_morkrut_banshee_fizzles_on_illegal_target
Source ticket: morkrut_banshee-01
Implementation: (not yet written)
Scenario: Morkrut Banshee ETBs with morbid targeting a creature for -4/-4. Give that target protection from black in response. Verify the ModifyPT effect does not apply.

### test_snapcaster_trigger_fizzles_if_target_exiled
Source ticket: snapcaster_mage-02
Implementation: (not yet written)
Scenario: Snapcaster ETBs targeting an instant in graveyard. Exile that instant in response. Verify the flashback grant does not apply (no GrantFlashback entry is pushed).

