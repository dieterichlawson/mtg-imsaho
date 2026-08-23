---
id: moldgraf_monstrosity-02
status: new
card: Moldgraf Monstrosity
audit_run_id: 2026-04-19-moldgraf_monstrosity-audit
audit_model: sonnet
audit_tokens: 28519
audit_duration: 480
---

## Audit Finding

**Oracle text:**
> return two creature cards at random from your graveyard to the battlefield

**Code:**
> state.move_object(*cid, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(*cid) {
                obj.controller = controller;
            }

**Description:**
The controller is assigned to the returned creature after `move_object` is called, but `move_object` emits the `EnteredBattlefield` event synchronously during the call (state.rs line 656), reading `obj.controller` at move time. If any creature in the graveyard carries a stale `controller` value — which occurs when a creature was previously stolen (e.g., Act of Treason), died while under an opponent's control, and returned to its owner's graveyard with the theft controller still set (because `move_object`'s leave-battlefield cleanup does not reset `controller` to `owner`, per the 'Controller field not reset to owner' insight) — the `EnteredBattlefield` event fires with the wrong controller. Any `AnyCreatureEnters` (EnterWatch) triggered ability that checks `entered_controller` will see the thief's player ID instead of the Monstrosity's controller. The fix is to set `obj.controller = controller` on the graveyard object before the `move_object` call so the correct value is already in place when the ETB event is emitted.

**Engine path:** mtg-engine/src/cards/isd/moldgraf_monstrosity.rs:70

## Tests

### moldgraf_returned_creature_etb_fires_with_monstrosity_controller
Scenario: A creature card owned by P0 has a stale controller of P1 in the graveyard (simulating a previous steal-then-die sequence); P0's Moldgraf Monstrosity dies and the trigger returns that creature to the battlefield; the EnteredBattlefield event should record P0 as the controller, not the stale P1.

