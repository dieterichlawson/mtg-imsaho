---
id: moldgraf_monstrosity-02
status: new
card: Moldgraf Monstrosity
card_file: mtg-engine/src/cards/isd/moldgraf_monstrosity.rs
created: 2026-04-15T03:44:23Z
audit_run_id: 2026-04-14-moldgraf_monstrosity-audit
audit_model: opus
audit_tokens: 15825
audit_duration: 317
---

## Audit Finding

**Oracle text:**
> return two creature cards at random from your graveyard to the battlefield.

**Code:**
> `moldgraf_monstrosity.rs:70-72`:
> ```rust
> state.move_object(*cid, Zone::Battlefield, registry);
> if let Some(obj) = state.get_object_mut(*cid) {
>     obj.controller = controller;
> }
> ```

**Description:**
The code sets `obj.controller` AFTER calling `state.move_object()`. The `move_object` function (state.rs:617-621) emits an `EnteredBattlefield` event using the object's controller at move time — which is the stale value from the graveyard (the creature's last controller on the battlefield, not necessarily the Moldgraf's controller). The explicit `obj.controller = controller` on line 72 corrects the field afterward, but the already-emitted `EnteredBattlefield` event carries the wrong controller. This means ETB triggers on the returned creatures (e.g., "When this creature enters the battlefield under your control") will see the wrong controller in the event data. The fix is to set `obj.controller = controller` BEFORE calling `move_object`, so the event captures the correct value.

**Engine path:**
- mtg-engine/src/cards/isd/moldgraf_monstrosity.rs:70-72
- mtg-engine/src/state.rs:617-621

**Required check:** 8b (trigger dispatch — EnteredBattlefield event controller)

**Affected cards:**
- Moldgraf Monstrosity
- Any other card that sets controller after `move_object` to the battlefield (known engine-wide pattern per auditor-insights.md)

## Tests

### moldgraf_returned_creature_etb_has_correct_controller
Source ticket: (new)
Implementation: (not yet written)
Scenario: Create a creature owned by P0 but last controlled by P1 (simulating a stolen-then-sacrificed creature) in P0's graveyard. Create Moldgraf Monstrosity controlled by P0. Kill Moldgraf and trigger `on_dies`. Assert the returned creature is on the battlefield controlled by P0. Capture the `EnteredBattlefield` event and assert its `controller` field is P0 (not the stale P1).

