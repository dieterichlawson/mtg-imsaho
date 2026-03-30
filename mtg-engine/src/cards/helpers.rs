//! Shared helper functions for common card resolution patterns.
//!
//! Includes:
//! - Spell resolution helpers (resolve_aura, resolve_damage, resolve_destroy)
//! - Choice presentation helpers (present_target_choice, present_yes_no, etc.)
//! - Target collection helpers (any_targets, creature_targets, etc.)

use crate::actions::Target;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::Zone;

/// Resolve an aura spell: attach it to the target creature on the battlefield.
/// If the target is no longer on the battlefield, the aura goes to graveyard.
/// Returns true if the aura was successfully attached.
pub fn resolve_aura(state: &mut GameState, aura_id: ObjectId, targets: &[Target]) -> bool {
    if let Some(Target::Object(target_id)) = targets.first() {
        if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.move_object(aura_id, Zone::Battlefield);
            if let Some(obj) = state.get_object_mut(aura_id) {
                obj.attached_to = Some(*target_id);
                obj.summoning_sick = false;
            }
            return true;
        }
    }
    state.move_spell_after_resolve(aura_id);
    false
}

/// Resolve a damage spell: deal `amount` damage to the first target
/// (creature or player), then move the spell to the appropriate zone.
pub fn resolve_damage(state: &mut GameState, spell_id: ObjectId, targets: &[Target], amount: u32) {
    if let Some(target) = targets.first() {
        match target {
            Target::Object(target_id) => {
                if let Some(obj) = state.get_object_mut(*target_id) {
                    if obj.zone == Zone::Battlefield {
                        obj.damage_marked += amount;
                        state.events.push(GameEvent::CombatDamageDealt {
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
                state.events.push(GameEvent::CombatDamageDealt {
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
    state.move_spell_after_resolve(spell_id);
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
    state.move_spell_after_resolve(spell_id);
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

/// All creatures on the battlefield.
pub fn creature_targets(state: &GameState) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All creatures on the battlefield except a specific one.
pub fn creature_targets_except(state: &GameState, exclude: ObjectId) -> Vec<Target> {
    state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.id != exclude)
        .map(|o| Target::Object(o.id))
        .collect()
}

/// All creatures + all players ("any target").
pub fn any_targets(state: &GameState) -> Vec<Target> {
    let mut targets = creature_targets(state);
    for player in &state.players {
        targets.push(Target::Player(player.id));
    }
    targets
}

/// All creatures + all players, excluding a specific creature.
pub fn any_targets_except(state: &GameState, exclude: ObjectId) -> Vec<Target> {
    let mut targets = creature_targets_except(state, exclude);
    for player in &state.players {
        targets.push(Target::Player(player.id));
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
    state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0))
}
