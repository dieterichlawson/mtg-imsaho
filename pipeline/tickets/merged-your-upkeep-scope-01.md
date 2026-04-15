---
id: merged-your-upkeep-scope-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: angel_of_flight_alabaster-01, bloodgift_demon-02, mayor_of_avabruck-03, splinterfright-01
---

# "At the beginning of your upkeep" fires during all players' upkeeps (CR 603.2)

## Description
Oracle text "at the beginning of your upkeep" (and analogously "your end step") constrains the trigger event to the controller's step. Per CR 603.2, the trigger fires only when its trigger event occurs. The engine's step-started dispatch (`triggers.rs:815-862` for upkeep, `triggers.rs:842+` for end step) queues an UpkeepTrigger / EndStepTrigger for every permanent with a matching `TriggerKind`, regardless of whose step it is. Card handlers compensate by early-returning if `state.active_player != controller`, but the trigger is still created, placed on the stack, and observable. The `TriggerKind` enum has no "your" vs "each" scope — it is an engine-level limitation.

## Engine path
- triggers.rs:815-862 (upkeep trigger dispatch — no controller filter)
- triggers.rs:860 (nap_triggers pushed for non-active-player permanents)
- triggers.rs:1123 (nap_triggers extended into pending_trigger_pushes_nap)

## Tests

### test_angel_of_flight_alabaster_trigger_only_on_your_upkeep
Source ticket: angel_of_flight_alabaster-01
Implementation: (not yet written)
Scenario: Angel on battlefield; opponent's upkeep begins. Verify no UpkeepTrigger for the Angel is on the stack. Controller's upkeep then begins — trigger should appear.

### test_bloodgift_demon_trigger_only_on_your_upkeep
Source ticket: bloodgift_demon-02
Implementation: (not yet written)
Scenario: Bloodgift Demon on battlefield; opponent's upkeep begins. Verify no draw-or-damage trigger is created.

### test_howlpack_alpha_end_step_trigger_only_on_your_end_step
Source ticket: mayor_of_avabruck-03
Implementation: (not yet written)
Scenario: Howlpack Alpha (transformed Mayor of Avabruck) on battlefield; opponent's end step begins. Verify no wolf-token trigger is on the stack.

### test_splinterfright_trigger_only_on_your_upkeep
Source ticket: splinterfright-01
Implementation: (not yet written)
Scenario: Splinterfright on battlefield; opponent's upkeep begins. Verify no mill trigger is on the stack.

