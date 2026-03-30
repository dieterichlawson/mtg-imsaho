//! Unified destruction pipeline.
//!
//! All "destroy" effects and state-based destruction flow through `try_destroy`,
//! which checks indestructible and regeneration before actually killing a permanent.
//! Sacrifice uses `sacrifice`, which bypasses both.

use crate::cards::CardRegistry;
use crate::events::GameEvent;
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::{Keyword, Zone};

/// Result of attempting to destroy a permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyResult {
    /// Permanent was destroyed (moved to graveyard).
    Died,
    /// Destruction prevented by indestructible (no state change).
    Indestructible,
    /// Destruction replaced by regeneration (state changed: tapped, damage cleared).
    Regenerated,
}

/// Attempt to destroy a permanent.
///
/// Pipeline:
/// 1. Indestructible — prevents destruction entirely (no state change).
/// 2. Regeneration shields — replaces destruction (tap, remove damage, consume shield).
/// 3. Falls through — permanent is destroyed (moved to graveyard).
///
/// Called by destroy spells (Doom Blade, etc.) and by SBAs for lethal damage / deathtouch.
/// NOT called for 0-toughness deaths (rule 704.5f) — those are not destruction.
pub fn try_destroy(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> DestroyResult {
    // Indestructible prevents destruction.
    if state.has_keyword(id, Keyword::Indestructible, registry) {
        return DestroyResult::Indestructible;
    }

    // Regeneration replaces destruction.
    let shields = state.get_object(id).map(|o| o.regeneration_shields).unwrap_or(0);
    if shields > 0 {
        regenerate(state, id);
        return DestroyResult::Regenerated;
    }

    // Actually destroy.
    destroy(state, id);
    DestroyResult::Died
}

/// Sacrifice a permanent. Bypasses indestructible and regeneration.
/// Returns true if the permanent existed and was sacrificed.
pub fn sacrifice(state: &mut GameState, id: ObjectId) -> bool {
    let exists = state.get_object(id)
        .map(|o| o.zone == Zone::Battlefield)
        .unwrap_or(false);
    if !exists {
        return false;
    }
    destroy(state, id);
    true
}

/// Apply regeneration: tap, remove damage, consume one shield, remove from combat.
fn regenerate(state: &mut GameState, id: ObjectId) {
    if let Some(obj) = state.get_object_mut(id) {
        obj.tapped = true;
        obj.damage_marked = 0;
        obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
        obj.regeneration_shields -= 1;
    }
    remove_from_combat(state, id);
    state.log(LogLevel::Event, format!("{} regenerated",
        state.get_object(id).map(|o| o.name.as_str()).unwrap_or("?")));
}

/// Actually destroy a permanent: emit events, move to graveyard, set morbid flag.
fn destroy(state: &mut GameState, id: ObjectId) {
    let is_creature = state.get_object(id).map(|o| o.power.is_some()).unwrap_or(false);
    if is_creature {
        let (cid, ctrl) = state.get_object(id)
            .map(|o| (o.card_id, o.controller))
            .unwrap_or((crate::ids::CardId(0), crate::ids::PlayerId(0)));
        state.events.push(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl });
        state.creature_died_this_turn = true;
    }
    state.move_object(id, Zone::Graveyard);
}

/// Remove a creature from the current combat (if any).
/// Used by regeneration and other effects that pull a creature out of combat.
pub fn remove_from_combat(state: &mut GameState, id: ObjectId) {
    if let Some(ref mut combat) = state.combat {
        combat.attackers.remove(&id);
        combat.blocker_assignments.remove(&id);
        for blockers in combat.blocker_assignments.values_mut() {
            blockers.retain(|&b| b != id);
        }
    }
}
