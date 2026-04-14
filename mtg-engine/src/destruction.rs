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
    let shields = state.get_object(id).map_or(0, |o| o.regeneration_shields);
    if shields > 0 {
        regenerate(state, id);
        return DestroyResult::Regenerated;
    }

    // Actually destroy.
    destroy(state, id, Some(registry));
    DestroyResult::Died
}

/// Destroy a permanent, bypassing regeneration ("can't be regenerated").
/// Still respects indestructible.
pub fn try_destroy_no_regen(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> DestroyResult {
    if state.has_keyword(id, Keyword::Indestructible, registry) {
        return DestroyResult::Indestructible;
    }
    destroy(state, id, Some(registry));
    DestroyResult::Died
}

/// Sacrifice a permanent. Bypasses indestructible and regeneration.
/// Returns true if the permanent existed and was sacrificed.
pub fn sacrifice(state: &mut GameState, id: ObjectId, registry: &CardRegistry) -> bool {
    let exists = state.get_object(id)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    if !exists {
        return false;
    }
    destroy(state, id, Some(registry));
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
        state.obj_name(id)));
}

/// Actually destroy a permanent: emit events, move to graveyard, set morbid flag.
fn destroy(state: &mut GameState, id: ObjectId, registry: Option<&CardRegistry>) {
    let is_creature = state.get_object(id).is_some_and(|o| o.power.is_some());
    if is_creature {
        // Capture last-known information before the zone change clears it.
        let (cid, ctrl, damaged_by) = state.get_object(id)
            .map_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new()), |o| (o.card_id, o.controller, o.damaged_by.clone()));
        let last_known_toughness = registry
            .and_then(|r| state.effective_toughness(id, r))
            .or_else(|| state.get_object(id).and_then(|o| o.toughness))
            .unwrap_or(0);
        state.events.push(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl, damaged_by, last_known_toughness });
        state.creature_died_this_turn = true;
    }
    // move_object handles the death/graveyard log message.
    state.move_object(id, Zone::Graveyard, registry.expect("registry required for move_object"));
}

/// Regenerate during SBA processing (public for sba.rs).
/// Skips the indestructible check since SBAs snapshot that separately.
pub fn regenerate_sba(state: &mut GameState, id: ObjectId) {
    regenerate(state, id);
}

/// Destroy during SBA processing (public for sba.rs).
/// Skips the indestructible check since SBAs snapshot that separately.
pub fn destroy_sba(state: &mut GameState, id: ObjectId, registry: &CardRegistry) {
    destroy(state, id, Some(registry));
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
