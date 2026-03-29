use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::state::{GameState, LogLevel};
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

    let name = registry.card_data(card_id).map(|d| d.name).unwrap_or_else(|| "?".into());
    state.log(LogLevel::Event, format!("{} resolved", name));
    state.events.push(GameEvent::SpellResolved { object: object_id });

    // Call the card's on_resolve behavior with targets.
    if let Some(behavior) = registry.get(card_id) {
        behavior.on_resolve(state, object_id, &targets);
    }

    // If the card set an awaiting_action, it's mid-resolution (e.g., Unburial
    // Rites waiting for player to choose a creature). Don't clean up yet —
    // the ResolveChoice handler in submit_action will do that.
    if state.awaiting_action.is_some() {
        return;
    }

    // If the card is still on the stack after resolution, move it to the
    // appropriate zone. Flashback spells go to exile; others to graveyard.
    if let Some(obj) = state.get_object(object_id) {
        if obj.zone == Zone::Stack {
            state.move_spell_after_resolve(object_id);
        }
    }
}
