---
id: fiend_hunter-02
status: new
card: Fiend Hunter
audit_run_id: 2026-04-19-fiend_hunter-audit
audit_model: sonnet
audit_tokens: 39764
audit_duration: 801
---

## Audit Finding

**Oracle text:**
> return the exiled card to the battlefield under its owner's control.

**Code:**
> // fiend_hunter.rs:66-70
state.move_object(target_id, Zone::Battlefield, registry);
// "under its owner's control" — reset controller to owner
if let Some(obj) = state.get_object_mut(target_id) {
    obj.controller = obj.owner;
}

**Description:**
In `on_leave_battlefield`, `move_object` is called before the controller field is corrected to the owner. The `move_object` function captures the object's current controller and emits an `EnteredBattlefield { controller }` event at that moment (state.rs). For any creature that was under a non-owner's control when Fiend Hunter exiled it (e.g., stolen via Act of Treason), the controller field in exile equals the thief, not the owner. When `move_object` fires, the `EnteredBattlefield` event carries that stale controller. Any `EnterWatch` trigger created from this event captures the wrong `entered_controller` value. Setting `obj.controller = obj.owner` afterward fixes the object's actual state but does not retroactively correct the already-emitted event. Per the engine's own 'Controller update after move_object' pattern, the controller should be set to the owner BEFORE calling `move_object` so the event carries the correct value.

**Engine path:** mtg-engine/src/cards/isd/fiend_hunter.rs:66-70

**Required check:** 8j

## Tests

### fiend_hunter_ltb_return_stale_entered_controller
Scenario: Opponent's creature is stolen with Act of Treason, then exiled by Fiend Hunter; when Fiend Hunter leaves and returns the creature, any watcher with 'whenever a creature enters under your control' belonging to the owner should fire for the owner, not for the player who stole it; the stale entered_controller in the EnterWatch event would cause this to misfire

