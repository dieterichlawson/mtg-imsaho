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

/// `try_destroy`, with one accurate line in the log naming what tried.
///
/// The pipeline already announces what *happened* — `move_object` writes the
/// death, `regenerate` writes the regeneration — but neither names the source,
/// and five cards wrote their own "X destroyed Y" line beside it without
/// looking at the result. Ghost Quarter's ruling is explicit that the land can
/// survive ("even if that land wasn't destroyed... because the land has
/// indestructible or because it was regenerated"), and the log said it was
/// destroyed anyway. This is the same shape as `mill_cards` taking a source:
/// the line that names the card is the one a reader trusts, so it has to be
/// the true one.
pub fn try_destroy_by(
    state: &mut GameState,
    id: ObjectId,
    source: &str,
    registry: &CardRegistry,
) -> DestroyResult {
    let name = state.obj_name(id);
    let result = try_destroy(state, id, registry);
    let line = match result {
        DestroyResult::Died => format!("{source} destroyed {name}"),
        DestroyResult::Regenerated => format!("{source} could not destroy {name} — it regenerated"),
        DestroyResult::Indestructible => format!("{source} could not destroy {name} — it is indestructible"),
    };
    state.log(crate::state::LogLevel::Event, line);
    result
}

/// Destroy several permanents simultaneously (CR 700.2c, CR 701.7b).
///
/// "Destroy all creatures" is one event, not a sequence of them, and the
/// difference is observable. Angelic Overseer is "indestructible as long as
/// you control a Human"; when a Wrath catches the Overseer and its last Human
/// together, the Human is still on the battlefield at the moment destruction
/// happens, so the Overseer survives. A loop over `try_destroy` gets that
/// wrong whenever it reaches the Human first — the Overseer's condition is
/// gone by the time its own check runs, and the Overseer dies too.
///
/// So this decides for every permanent against the same game state, the one
/// before any of them has died, and captures each death's last known
/// information there as well (CR 608.2g) before moving anything. Results come
/// back in the order given.
pub fn try_destroy_all(
    state: &mut GameState,
    ids: &[ObjectId],
    registry: &CardRegistry,
) -> Vec<(ObjectId, DestroyResult)> {
    // Phase 1 — decide. Nothing has moved yet, so every check sees the same
    // battlefield.
    let decisions: Vec<(ObjectId, DestroyResult)> = ids.iter()
        .map(|&id| {
            let result = if state.has_keyword(id, Keyword::Indestructible, registry) {
                DestroyResult::Indestructible
            } else if state.get_object(id).is_some_and(|o| o.regeneration_shields > 0) {
                DestroyResult::Regenerated
            } else {
                DestroyResult::Died
            };
            (id, result)
        })
        .collect();

    // Phase 2 — capture the death events, still against that same state, so a
    // creature whose toughness depends on the others (Splinterfright counting
    // creature cards in the graveyard) is remembered as it was.
    let deaths: Vec<(ObjectId, Option<GameEvent>)> = decisions.iter()
        .filter(|(_, r)| *r == DestroyResult::Died)
        .map(|&(id, _)| (id, death_event(state, id, Some(registry))))
        .collect();

    // Phase 3 — apply.
    for &(id, result) in &decisions {
        if result == DestroyResult::Regenerated {
            regenerate(state, id);
        }
    }
    for (id, event) in deaths {
        if let Some(event) = event {
            state.events.push(event);
            state.creature_died_this_turn = true;
        }
        state.move_object(id, Zone::Graveyard, registry);
    }

    decisions
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
    state.tap(id);
    if let Some(obj) = state.get_object_mut(id) {
        obj.damage_marked = 0;
        obj.dealt_deathtouch_damage = false; obj.damaged_by.clear();
        obj.regeneration_shields -= 1;
    }
    remove_from_combat(state, id);
    state.log(LogLevel::Event, format!("{} regenerated",
        state.obj_name(id)));
}

/// The `CreatureDied` event for a permanent about to be destroyed, built from
/// last known information (CR 608.2g) — so it has to be called BEFORE the zone
/// change, which clears the object's battlefield state. `None` for a
/// non-creature, which announces no death.
fn death_event(state: &GameState, id: ObjectId, registry: Option<&CardRegistry>) -> Option<GameEvent> {
    let is_creature = registry.is_some_and(|r| state.is_creature(id, r))
        || state.get_object(id).is_some_and(|o| o.power.is_some());
    if !is_creature {
        return None;
    }
    let (cid, ctrl, damaged_by, is_token) = state.get_object(id)
        .map_or((crate::ids::CardId(0), crate::ids::PlayerId(0), Vec::new(), false), |o| (o.card_id, o.controller, o.damaged_by.clone(), o.is_token));
    let last_known_toughness = registry
        .and_then(|r| state.effective_toughness(id, r))
        .or_else(|| state.get_object(id).and_then(|o| o.toughness))
        .unwrap_or(0);
    Some(GameEvent::CreatureDied { object: id, card_id: cid, controller: ctrl, damaged_by, last_known_toughness, is_token })
}

/// Actually destroy a permanent: emit events, move to graveyard, set morbid flag.
fn destroy(state: &mut GameState, id: ObjectId, registry: Option<&CardRegistry>) {
    if let Some(event) = death_event(state, id, registry) {
        state.events.push(event);
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
