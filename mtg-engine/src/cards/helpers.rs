//! Shared helper functions for common card resolution patterns.
//!
//! These reduce boilerplate across card implementations for the three
//! most common on_resolve patterns: aura attachment, damage dealing,
//! and targeted destruction.

use crate::actions::Target;
use crate::events::{DamageTarget, GameEvent};
use crate::ids::ObjectId;
use crate::state::GameState;
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
    state.move_object(aura_id, Zone::Graveyard);
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
