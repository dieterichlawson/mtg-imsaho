use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::state::{GameState, LogLevel, StackEntry};
use crate::types::Zone;

/// Check if a target is still legal at resolution time.
fn is_target_legal(state: &GameState, target: &Target) -> bool {
    match target {
        Target::Object(id) => {
            match state.get_object(*id) {
                Some(obj) => {
                    // Creature/permanent targets must be on the battlefield.
                    // Stack targets (for counterspells) must be on the stack.
                    obj.zone == Zone::Battlefield || obj.zone == Zone::Stack
                }
                None => false, // Object doesn't exist.
            }
        }
        Target::Player(_) => true, // Players are always legal targets in 2-player.
    }
}

/// Resolve the top item on the stack (spell or trigger).
///
/// For spells: checks target legality (CR 608.2b fizzle), calls on_resolve.
/// For triggers: delegates to triggers::resolve_next_trigger.
pub fn resolve_top_of_stack(state: &mut GameState, registry: &CardRegistry) {
    let entry = match state.stack.last() {
        Some(e) => e.clone(),
        None => return,
    };

    match entry {
        StackEntry::Trigger(_) => {
            // Trigger resolution is handled by the triggers module.
            crate::triggers::resolve_next_trigger(state, registry);
        }
        StackEntry::Spell(object_id) => {
            state.stack.pop(); // Remove the spell from the stack.
            resolve_spell(state, registry, object_id);
        }
    }
}

/// Resolve a spell from the stack.
fn resolve_spell(state: &mut GameState, registry: &CardRegistry, object_id: crate::ids::ObjectId) {
    let (card_id, targets) = match state.get_object(object_id) {
        Some(obj) => (obj.card_id, obj.targets.clone()),
        None => return,
    };

    let name = registry.card_data(card_id).map(|d| d.name).unwrap_or_else(|| "?".into());

    // CR 608.2b: Check target legality. If the spell has targets and ALL
    // are illegal, it's countered by game rules (fizzled).
    if !targets.is_empty() {
        let any_legal = targets.iter().any(|t| is_target_legal(state, t));
        if !any_legal {
            state.log(LogLevel::Event, format!("{} fizzled (all targets illegal)", name));
            // Move to graveyard (or exile for flashback) without resolving.
            state.move_spell_after_resolve(object_id);
            return;
        }
    }

    // Spell resolves normally.
    state.log(LogLevel::Event, format!("{} resolved", name));
    state.events.push(GameEvent::SpellResolved { object: object_id });

    // Call the card's on_resolve behavior with targets.
    if let Some(behavior) = registry.get(card_id) {
        behavior.on_resolve(state, object_id, &targets, registry);
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
