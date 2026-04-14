use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::state::{GameState, LogLevel, StackEntry};
use crate::types::Zone;

/// Check if a target is still legal at resolution time.
fn is_target_legal(state: &GameState, target: &Target, target_req: &crate::cards::TargetRequirement, caster: crate::ids::PlayerId, registry: &crate::cards::CardRegistry) -> bool {
    use crate::cards::TargetRequirement;

    // ModalChoice: legal if legal under any mode.
    if let TargetRequirement::ModalChoice(ref modes) = target_req {
        return modes.iter().any(|mode_req| is_target_legal(state, target, mode_req, caster, registry));
    }

    // Unwrap nested requirements (UpToTargets, TwoTargets).
    let inner_req = match target_req {
        TargetRequirement::UpToTargets(_, inner) => inner.as_ref(),
        TargetRequirement::TwoTargets(inner, _) => inner.as_ref(),
        other => other,
    };
    match target {
        Target::Object(id) => {
            match state.get_object(*id) {
                Some(obj) => {
                    // Check zone legality.
                    let zone_ok = match inner_req {
                        TargetRequirement::GraveyardCard
                        | TargetRequirement::GraveyardCreature
                        | TargetRequirement::GraveyardCreatureOfSubtype(_)
                        | TargetRequirement::GraveyardCardOwnedByCaster
                        | TargetRequirement::GraveyardCardOwnedByOpponent => obj.zone == Zone::Graveyard,
                        TargetRequirement::ExileCard => obj.zone == Zone::Exile,
                        _ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack,
                    };
                    if !zone_ok { return false; }

                    // Check hexproof: opponent's creature with hexproof can't be targeted.
                    if obj.zone == Zone::Battlefield && obj.controller != caster
                        && state.has_keyword(*id, crate::types::Keyword::Hexproof, registry) {
                        return false;
                    }

                    true
                }
                None => false,
            }
        }
        Target::Player(pid) => {
            // Check player hexproof (Witchbane Orb).
            if *pid != caster && state.player_has_hexproof(*pid, registry) {
                return false;
            }
            true
        }
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
    let (card_id, targets, caster) = match state.get_object(object_id) {
        Some(obj) => (obj.card_id, obj.targets.clone(), obj.controller),
        None => return,
    };

    // CR 608.2b: Check target legality. If the spell has targets and ALL
    // are illegal, it's countered by game rules (fizzled).
    // This now checks hexproof at resolution time (not just at cast time).
    let target_req = registry.get(card_id)
        .map_or(crate::cards::TargetRequirement::None, super::cards::CardBehavior::target_requirement);
    if !targets.is_empty() {
        let behavior = registry.get(card_id);
        let any_legal = targets.iter().any(|t| {
            if !is_target_legal(state, t, &target_req, caster, registry) {
                return false;
            }
            // Also re-check card-specific validity (e.g., "power 4 or greater").
            if let Some(b) = behavior {
                b.is_valid_target(state, caster, t, registry)
            } else {
                true
            }
        });
        if !any_legal {
            state.log(LogLevel::Event, format!("{} fizzled (all targets illegal)", state.obj_name(object_id)));
            // Move to graveyard (or exile for flashback) without resolving.
            state.move_spell_after_resolve(object_id, registry);
            return;
        }
    }

    // Spell resolves normally.
    state.log(LogLevel::Event, format!("{} resolved", state.obj_name(object_id)));
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
            state.move_spell_after_resolve(object_id, registry);
        }
    }
}
