use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::Zone;

/// Process triggered abilities based on events in state.events.
/// Called after submit_action and after SBAs to handle ETB, dies, and death-watch triggers.
/// Triggers resolve immediately unless they set an awaiting_action (mid-resolution choice),
/// in which case processing pauses and resumes from trigger_event_index next call.
pub fn process_triggers(state: &mut GameState, registry: &CardRegistry) {
    let events = state.events.clone();
    let start = state.trigger_event_index;

    for (i, event) in events.iter().enumerate().skip(start) {
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
            GameEvent::CreatureDied { object, card_id, controller } => {
                let dead_id = *object;
                let dead_card_id = *card_id;
                let dead_controller = *controller;

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
            GameEvent::LeftBattlefield { object, .. } => {
                let obj_id = *object;
                let card_id = match state.get_object(obj_id) {
                    Some(o) => o.card_id,
                    None => continue,
                };
                if let Some(behavior) = registry.get(card_id) {
                    behavior.on_leave_battlefield(state, obj_id, registry);
                }
            }
            _ => {}
        }

        // If a trigger set an awaiting_action, pause and resume later.
        if state.awaiting_action.is_some() {
            state.trigger_event_index = i + 1;
            return;
        }
    }

    // All events processed — reset index.
    state.trigger_event_index = 0;
}
