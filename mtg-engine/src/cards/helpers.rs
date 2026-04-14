//! Shared helper functions for common card resolution patterns.
//!
//! Includes:
//! - Spell resolution helpers (resolve_aura, resolve_damage, resolve_destroy)
//! - Choice presentation helpers (present_target_choice, present_yes_no, etc.)
//! - Target collection helpers (any_targets, creature_targets, etc.)

use crate::actions::Target;
use crate::cards::CardRegistry;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// Resolve an aura spell: attach it to the target creature on the battlefield.
/// If the target is no longer on the battlefield, the aura goes to graveyard.
/// Returns true if the aura was successfully attached.
pub fn resolve_aura(state: &mut GameState, aura_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Object(target_id)) = targets.first() {
        if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.move_object(aura_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(aura_id) {
                obj.attached_to = Some(*target_id);
                obj.summoning_sick = false;
            }
            return true;
        }
    }
    state.move_spell_after_resolve(aura_id, registry);
    false
}

/// Resolve a curse aura: attach to a target player and move to battlefield.
pub fn resolve_curse(state: &mut GameState, curse_id: ObjectId, targets: &[Target], registry: &CardRegistry) -> bool {
    if let Some(Target::Player(player_id)) = targets.first() {
        state.move_object(curse_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(curse_id) {
            obj.attached_to_player = Some(*player_id);
            obj.summoning_sick = false;
        }
        return true;
    }
    state.move_spell_after_resolve(curse_id, registry);
    false
}

/// Resolve a damage spell: deal `amount` damage to the first target
/// (creature or player), then move the spell to the appropriate zone.
pub fn resolve_damage(state: &mut GameState, spell_id: ObjectId, targets: &[Target], amount: u32, registry: &CardRegistry) {
    if let Some(target) = targets.first() {
        match target {
            Target::Object(target_id) => {
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        obj.damage_marked += amount;
                        obj.damaged_by.push(spell_id);
                        state.events.push(GameEvent::NonCombatDamageDealt {
                            source: spell_id,
                            target: DamageTarget::Object(*target_id),
                            amount,
                        });
                    }
                }
            }
            Target::Player(player_id) => {
                let old_life = state.get_player(*player_id).life;
                let new_life = old_life - (amount as i32);
                state.get_player_mut(*player_id).life = new_life;
                state.events.push(GameEvent::NonCombatDamageDealt {
                    source: spell_id,
                    target: DamageTarget::Player(*player_id),
                    amount,
                });
                state.events.push(GameEvent::LifeChanged {
                    player: *player_id,
                    old: old_life,
                    new_life,
                });
            }
        }
    }
    state.move_spell_after_resolve(spell_id, registry);
}

/// Resolve a targeted destruction spell: destroy the first target creature
/// via the destruction pipeline (checks indestructible/regeneration),
/// then move the spell to the appropriate zone.
pub fn resolve_destroy(
    state: &mut GameState,
    spell_id: ObjectId,
    targets: &[Target],
    registry: &crate::cards::CardRegistry,
) {
    if let Some(Target::Object(target_id)) = targets.first() {
        if let Some(obj) = state.get_object(*target_id) {
            if obj.zone == Zone::Battlefield {
                crate::destruction::try_destroy(state, *target_id, registry);
            }
        }
    }
    state.move_spell_after_resolve(spell_id, registry);
}

// ═══════════════════════════════════════════════════════════════════
// Choice presentation helpers
//
// These set up AwaitingAction::ResolutionChoice so the game loop
// asks the player to make a decision. The CLI and LLM player both
// know how to render these choices.
// ═══════════════════════════════════════════════════════════════════

/// Present a "choose one target" choice to the player.
///
/// - If `targets` is empty, does nothing.
/// - If mandatory (`optional == false`) and exactly 1 target, auto-applies the effect.
/// - Otherwise, sets up a ResolutionChoice for the player to pick.
pub fn present_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
    optional: bool,
) {
    if targets.is_empty() {
        return;
    }
    if targets.len() == 1 && !optional {
        // Mandatory with exactly 1 target — auto-apply.
        let reg = crate::cards::CardRegistry::with_all_cards();
        crate::engine::apply_pending_effect(state, &targets[0], &effect, &reg);
        return;
    }
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: controller,
        source: source_id,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: description.into(),
            options: targets,
            optional,
            effect,
        },
    });
}

/// Present a "choose one target" choice that is optional ("you may").
pub fn present_optional_target_choice(
    state: &mut GameState,
    source_id: ObjectId,
    controller: PlayerId,
    targets: Vec<Target>,
    effect: PendingEffect,
    description: &str,
) {
    present_target_choice(state, source_id, controller, targets, effect, description, true);
}

// ═══════════════════════════════════════════════════════════════════
// Target collection helpers
//
// Build lists of valid targets for common patterns.
// ═══════════════════════════════════════════════════════════════════

/// All targetable creatures on the battlefield (respects hexproof/protection).
pub fn creature_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All targetable creatures on the battlefield except a specific one.
pub fn creature_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != exclude)
        .filter(|o| crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry))
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All targetable creatures + planeswalkers + all players ("any target").
pub fn any_targets(state: &GameState, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets(state, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.objects.values() {
        if o.zone == Zone::Battlefield && o.power.is_none()
            && o.card_types.contains(&crate::types::CardType::Planeswalker)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if !state.player_has_hexproof(player.id, registry) || player.id == controller {
            targets.push(Target::Player(player.id));
        }
    }
    targets
}

/// All targetable creatures + planeswalkers + all players, excluding a specific object.
pub fn any_targets_except(state: &GameState, exclude: ObjectId, source_id: ObjectId, controller: PlayerId, registry: &CardRegistry) -> Vec<Target> {
    let mut targets = creature_targets_except(state, exclude, source_id, controller, registry);
    // Add planeswalkers (which have power = None, so creature_targets misses them)
    for o in state.objects.values() {
        if o.zone == Zone::Battlefield && o.power.is_none() && o.id != exclude
            && o.card_types.contains(&crate::types::CardType::Planeswalker)
            && crate::engine::can_be_targeted_by(state, o.id, controller, Some(source_id), registry)
        {
            targets.push(Target::Object(o.id));
        }
    }
    for player in &state.players {
        if !state.player_has_hexproof(player.id, registry) || player.id == controller {
            targets.push(Target::Player(player.id));
        }
    }
    targets
}

/// All creatures controlled by a specific player.
pub fn creatures_controlled_by(state: &GameState, player: PlayerId) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.controller == player)
        .map(|o| Target::Object(o.id))
        .collect()
}

/// The single opponent in a 2-player game (auto-target convenience).
pub fn opponent_player(state: &GameState, controller: PlayerId) -> Target {
    Target::Player(state.opponent(controller))
}

/// Get the controller of a permanent, with a fallback.
pub fn controller_of(state: &GameState, object_id: ObjectId) -> PlayerId {
    state.get_object(object_id).map_or(PlayerId(0), |o| o.controller)
}

// ═══════════════════════════════════════════════════════════════════
// Transform helpers
//
// Generic transform logic for double-faced cards. Updates all
// face-dependent properties (name, keywords, subtypes) so that
// obj.keywords always reflects the active face.
// ═══════════════════════════════════════════════════════════════════

/// Toggle `is_transformed` on a permanent and update all face-dependent
/// properties (name, keywords, subtypes) to match the new active face.
///
/// This is the correct way to transform a DFC. Card-specific code should
/// call this instead of manually flipping `obj.is_transformed` and `obj.name`,
/// which leaves `obj.keywords` and `obj.subtypes` stale.
pub fn apply_transform(state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
    let (card_id, was_transformed) = match state.get_object(object_id) {
        Some(o) if o.zone == Zone::Battlefield => (o.card_id, o.is_transformed),
        _ => return,
    };

    let Some(behavior) = registry.get(card_id) else { return; };

    // Flip the transform flag.
    if let Some(obj) = state.get_object_mut(object_id) {
        obj.is_transformed = !was_transformed;
    }

    if was_transformed {
        // Was on back face -> now front face. Restore front face data.
        let front = behavior.card_data();
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.name.clone_from(&front.name);
            obj.keywords.clone_from(&front.keywords);
            obj.subtypes.clone_from(&front.subtypes);
        }
    } else {
        // Was on front face -> now back face. Apply back face data.
        if let Some(back) = behavior.back_face_data() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.name.clone_from(&back.name);
                obj.keywords.clone_from(&back.keywords);
                obj.subtypes.clone_from(&back.subtypes);
            }
        }
    }
}
