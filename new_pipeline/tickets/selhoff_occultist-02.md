---
id: selhoff_occultist-02
status: new
card: Selhoff Occultist
audit_run_id: 2026-04-19-selhoff_occultist-audit
audit_model: sonnet
audit_tokens: 21123
audit_duration: 374
---

## Audit Finding

**Oracle text:**
> Whenever this creature or another creature dies, target player mills a card

**Code:**
> fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, ...) {
    let controller = match state.get_object(self_id) {
        Some(o) if o.zone == Zone::Battlefield => o.controller,
        _ => return,
    };
    present_mill_choice(state, self_id, controller, registry);
}

**Description:**
The handler bails out with `return` whenever Selhoff Occultist is not in Zone::Battlefield at resolution time. When Selhoff Occultist dies simultaneously with another creature (e.g., a board wipe kills both), the engine correctly creates a DeathWatch trigger for the Occultist watching the other creature die — the `simultaneously_dead` collection at triggers.rs:647–653 explicitly includes creature objects that died in the same event batch. The comment at triggers.rs:1341 states: 'Per MTG rules, death triggers fire even if the watcher died simultaneously (e.g., Falkenrath Noble + board wipe). The trigger was created when the watcher was last known to be on the battlefield.' By the time this trigger resolves, however, the Occultist is in the graveyard, so `o.zone == Zone::Battlefield` is false and the handler returns without milling. The oracle text's 'another creature dies' arm should fire regardless of whether the Occultist itself survived. The fix is to read the controller from the graveyard object without requiring the battlefield zone, or to use the controller already stored in the PendingTrigger::DeathWatch struct.

**Engine path:** mtg-engine/src/cards/isd/selhoff_occultist.rs:51

**Required check:** 8b

**Affected cards:**
- Lumberknot
- Unruly Mob
- Village Cannibals
- Rage Thrower

## Tests

### simultaneous_death_any_creature_dies_mills
Scenario: A board wipe destroys both Selhoff Occultist and another creature simultaneously; the Occultist's AnyCreatureDies trigger should resolve and mill one card even though the Occultist is no longer on the battlefield.

