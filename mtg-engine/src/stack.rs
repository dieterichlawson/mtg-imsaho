use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::state::GameState;
use crate::types::Zone;

/// Resolve the top item on the stack.
pub fn resolve_top_of_stack(state: &mut GameState, registry: &CardRegistry) {
    let object_id = match state.stack.pop() {
        Some(id) => id,
        None => return,
    };

    let (card_id, targets) = match state.get_object(object_id) {
        Some(obj) => (obj.card_id, obj.targets.clone()),
        None => return,
    };

    state.events.push(GameEvent::SpellResolved { object: object_id });

    // Call the card's on_resolve behavior with targets.
    if let Some(behavior) = registry.get(card_id) {
        behavior.on_resolve(state, object_id, &targets);
    }

    // If the card is still on the stack after resolution (instants/sorceries
    // should have moved themselves to graveyard, but catch any that didn't),
    // move it to graveyard.
    if let Some(obj) = state.get_object(object_id) {
        if obj.zone == Zone::Stack {
            state.move_object(object_id, Zone::Graveyard);
        }
    }
}
