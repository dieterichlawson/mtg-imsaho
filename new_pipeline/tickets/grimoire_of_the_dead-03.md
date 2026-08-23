---
id: grimoire_of_the_dead-03
status: fixed
card: Grimoire of the Dead
audit_run_id: 2026-04-19-grimoire_of_the_dead-audit
audit_model: sonnet
audit_tokens: 22452
audit_duration: 439
fixed_sha: 1e05ab6e4596cc9aab3a893ece318376dea011d6
fixed_at: 2026-08-23T20:24:12Z
test_file: mtg-engine/tests/enters_under_control.rs
fix_note: cluster fix: move_object_under_control sets the controller before the zone change so EnteredBattlefield carries it (CR 110.2)
---

## Audit Finding

**Oracle text:**
> Put all creature cards from all graveyards onto the battlefield under your control.

**Code:**
> state.move_object(cid, Zone::Battlefield, registry);
                    if let Some(obj) = state.get_object_mut(cid) {
                        obj.controller = controller;

**Description:**
For each creature card moved to the battlefield, move_object is called first, then obj.controller is set afterward. move_object emits the EnteredBattlefield event using the object's controller at the time of the move — which is the previous controller or owner, not the Grimoire controller. Per the 'Controller update after move_object causes stale EnteredBattlefield events' insight, the EnterWatch trigger's entered_controller field and any trigger dispatch code that reads controller from the event will see the wrong controller for all creatures reanimated this way. The fix is to set obj.controller = controller before calling move_object, so the event is emitted with the correct controller.

**Engine path:** mtg-engine/src/cards/isd/grimoire_of_the_dead.rs:157

**Required check:** 8a

## Tests

### etb_trigger_controller_correct_after_reanimate
Scenario: An opponent's creature with an enters-the-battlefield trigger is reanimated by Grimoire of the Dead. Verify that the ETB trigger's entered_controller field matches the Grimoire controller, not the opponent who owned the creature.

