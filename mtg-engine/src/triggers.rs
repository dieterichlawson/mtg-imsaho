use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::Zone;

/// Process triggered abilities based on events in state.events.
/// Called after submit_action and after SBAs to handle ETB, dies, and death-watch triggers.
/// Triggers resolve immediately (simplified — does not use the stack).
pub fn process_triggers(state: &mut GameState, registry: &CardRegistry) {
    // Snapshot events to avoid borrow issues (triggers may push new events).
    let events = state.events.clone();

    for event in &events {
        match event {
            GameEvent::EnteredBattlefield { object, .. } => {
                let obj_id = *object;
                let card_id = match state.get_object(obj_id) {
                    Some(o) if o.zone == Zone::Battlefield => o.card_id,
                    _ => continue,
                };
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_enter_battlefield(state, obj_id, registry);
                }
            }
            GameEvent::CreatureDied { object } => {
                let dead_id = *object;
                let (dead_card_id, dead_controller) = match state.get_object(dead_id) {
                    Some(o) => (o.card_id, o.controller),
                    None => continue,
                };

                // 1. Self-dies trigger: the creature that died.
                if let Some(behavior) = registry.get(dead_card_id) {
                    behavior.on_dies(state, dead_id, registry);
                }

                // 2. Death-watch: notify all permanents on the battlefield.
                let watchers: Vec<(ObjectId, crate::ids::CardId)> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)
                    .map(|o| (o.id, o.card_id))
                    .collect();
                for (watcher_id, watcher_card_id) in watchers {
                    if let Some(behavior) = registry.get(watcher_card_id) {
                        behavior.on_any_creature_dies(state, watcher_id, dead_id, dead_controller, registry);
                    }
                }
            }
            _ => {}
        }
    }
}
