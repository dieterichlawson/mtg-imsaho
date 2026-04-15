---
id: grimoire_of_the_dead-03
status: closed-duplicate
card: Grimoire of the Dead
card_file: mtg-engine/src/cards/isd/grimoire_of_the_dead.rs
created: 2026-04-14T20:57:12Z
audit_run_id: 2026-04-14-grimoire_of_the_dead-audit
audit_model: opus
audit_tokens: 16027
audit_duration: 412
duplicate_of: merged-controller-after-move-01
---

## Audit Finding

**Oracle text:**
> Put all creature cards from all graveyards onto the battlefield under your control.

**Code:**
> grimoire_of_the_dead.rs:157-159:
> ```
> state.move_object(cid, Zone::Battlefield, registry);
> if let Some(obj) = state.get_object_mut(cid) {
>     obj.controller = controller;
> ```
> state.rs:617-621 (inside move_object):
> ```
> let controller = self.get_object(id).map_or(PlayerId(0), |o| o.controller);
> self.events.push(crate::events::GameEvent::EnteredBattlefield {
>     object: id,
>     controller,
> });
> ```

**Description:**
The code calls `move_object(cid, Zone::Battlefield, registry)` first, then sets `obj.controller = controller` afterward. The `move_object` function emits an `EnteredBattlefield` event using the object's controller at move time — which is the creature's owner (its controller while in the graveyard), not the Grimoire's controller. Any trigger that reads the `entered_controller` field from the event (e.g., "Whenever a creature enters the battlefield under your control") would see the wrong controller. The controller should be set BEFORE the move, or `move_object` should accept a controller parameter.

**Engine path:**
- grimoire_of_the_dead.rs:157 (move_object before controller set)
- grimoire_of_the_dead.rs:159 (controller set after)
- state.rs:617-621 (event emitted with stale controller)

**Required check:** 8a (zone-change event correctness)

**Affected cards:**
- Grimoire of the Dead
- Any card that moves objects to the battlefield "under your control" then sets controller after move_object
