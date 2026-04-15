---
id: merged-controller-after-move-01
status: new
card: multiple
created: 2026-04-15T04:57:48Z
kind: consolidated
source_tickets: grimoire_of_the_dead-03, mirror_mad_phantasm-02, moldgraf_monstrosity-02
---

# EnteredBattlefield event emitted with stale controller before explicit reassignment

## Description
When a card moves a creature to the battlefield "under your control" by calling `state.move_object(id, Zone::Battlefield, registry)` and then assigning `obj.controller = controller` afterward, the `EnteredBattlefield` event emitted inside `move_object` (state.rs:617-621) carries the object's pre-move controller value — typically the creature's owner from the graveyard. Any ETB trigger that reads the event's controller field (e.g., "whenever a creature enters the battlefield under your control") fires for the wrong player or fails to fire for the correct one. The fix is to set `obj.controller` before calling `move_object`, or to add an optional controller parameter to `move_object`.

## Engine path
- state.rs:617-621 (EnteredBattlefield event emitted using current controller before reassignment)
- state.rs:572-583 (zone-change cleanup does not reset controller)

## Tests

### grimoire_etb_event_has_grimoire_controller
Source ticket: grimoire_of_the_dead-03
Implementation: (not yet written)
Scenario: Grimoire of the Dead's activated ability puts a creature from Player B's graveyard onto the battlefield under Player A's control. Verify the EnteredBattlefield event's controller field is Player A (the Grimoire's controller), not Player B (the creature's owner). Currently fails because move_object emits the event with Player B as controller.

### mirror_mad_phantasm_etb_controller_after_steal
Source ticket: mirror_mad_phantasm-02
Implementation: (not yet written)
Scenario: Player B controls Mirror-Mad Phantasm (owned by Player A) via a steal effect. Player B activates the ability; the Phantasm is shuffled into Player A's library and found. When it enters the battlefield, verify the EnteredBattlefield event's controller field is Player A (the owner). Currently fails because the event carries Player B (stale controller from before the shuffle).

### moldgraf_returned_creature_etb_controller
Source ticket: moldgraf_monstrosity-02
Implementation: (not yet written)
Scenario: A creature owned by Player A but last controlled by Player B (stolen then sacrificed) is in Player A's graveyard. Moldgraf Monstrosity controlled by Player A dies and returns the creature. Verify the EnteredBattlefield event's controller field is Player A, not the stale Player B.

